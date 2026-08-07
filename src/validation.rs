use crate::{
    graph::{EIGHT_WAY_DELTAS, adjacent_square},
    serialization::{
        MAX_CERTIFICATE_TEXT_BYTES, bitboard_bits, decomposition_rejection_reason_tag,
        decomposition_status_tag, push_len_prefixed_bytes, sha256,
    },
    types::*,
};
use std::collections::{BTreeSet, HashSet};

impl CompositionCertificate {
    /// Returns a versioned canonical byte payload for stable provenance labels.
    ///
    /// `BMCOMPOSE` v1 accepts at most 65,535 UTF-8 bytes in each value-digest
    /// field.
    pub fn canonical_payload(&self) -> Result<Vec<u8>, CompositionCertificateValidationError> {
        self.validate()?;
        Ok(self.canonical_payload_unchecked())
    }

    /// Returns a stable SHA-256 digest of this certificate's canonical payload.
    pub fn digest(
        &self,
    ) -> Result<CompositionCertificateDigest, CompositionCertificateValidationError> {
        Ok(CompositionCertificateDigest(sha256(
            &self.canonical_payload()?,
        )))
    }

    /// Validates structural invariants for this composition certificate.
    pub fn validate(&self) -> Result<(), CompositionCertificateValidationError> {
        if self.component_values.is_empty() {
            return Err(CompositionCertificateValidationError::EmptyComponentValues);
        }
        if self.result_value_digest.is_empty() {
            return Err(CompositionCertificateValidationError::EmptyResultValueDigest);
        }
        if self.result_value_digest.len() > MAX_CERTIFICATE_TEXT_BYTES {
            return Err(
                CompositionCertificateValidationError::ResultValueDigestTooLong {
                    actual: self.result_value_digest.len(),
                    maximum: MAX_CERTIFICATE_TEXT_BYTES,
                },
            );
        }

        let mut roots = HashSet::new();
        for component in &self.component_values {
            if component.value_digest.is_empty() {
                return Err(
                    CompositionCertificateValidationError::EmptyComponentValueDigest {
                        component_root: component.component_root,
                    },
                );
            }
            if component.value_digest.len() > MAX_CERTIFICATE_TEXT_BYTES {
                return Err(
                    CompositionCertificateValidationError::ComponentValueDigestTooLong {
                        component_root: component.component_root,
                        actual: component.value_digest.len(),
                        maximum: MAX_CERTIFICATE_TEXT_BYTES,
                    },
                );
            }
            if !roots.insert(component.component_root) {
                return Err(
                    CompositionCertificateValidationError::DuplicateComponentRoot {
                        component_root: component.component_root,
                    },
                );
            }
        }
        Ok(())
    }

    /// Validates this composition certificate against the concrete decomposition it cites.
    ///
    /// This is the checker downstream generators should run before promoting a
    /// composed row to exact supervision. It verifies that the referenced
    /// decomposition certificate is valid and strict, that its digest matches
    /// [`CompositionCertificate::decomposition_digest`], and that every strict
    /// decomposition component root has exactly one component value.
    pub fn validate_against_decomposition(
        &self,
        decomposition: &DecompositionCertificate,
    ) -> Result<(), CompositionCertificateValidationError> {
        self.validate()?;

        decomposition.validate().map_err(|error| {
            CompositionCertificateValidationError::InvalidDecompositionCertificate { error }
        })?;

        if decomposition.status != DecompositionStatus::Strict {
            return Err(
                CompositionCertificateValidationError::CompositionRequiresStrictDecomposition {
                    status: decomposition.status,
                },
            );
        }

        let certificate_digest = decomposition.digest().map_err(|error| {
            CompositionCertificateValidationError::InvalidDecompositionCertificate { error }
        })?;
        if certificate_digest != self.decomposition_digest {
            return Err(
                CompositionCertificateValidationError::DecompositionDigestMismatch {
                    certificate_digest,
                    composition_digest: self.decomposition_digest,
                },
            );
        }

        let expected_roots = decomposition
            .components
            .iter()
            .map(|component| component.root)
            .collect::<BTreeSet<_>>();
        let actual_roots = self
            .component_values
            .iter()
            .map(|component| component.component_root)
            .collect::<BTreeSet<_>>();

        for component_root in &expected_roots {
            if !actual_roots.contains(component_root) {
                return Err(
                    CompositionCertificateValidationError::MissingComponentRoot {
                        component_root: *component_root,
                    },
                );
            }
        }
        for component_root in actual_roots {
            if !expected_roots.contains(&component_root) {
                return Err(
                    CompositionCertificateValidationError::UnexpectedComponentRoot {
                        component_root,
                    },
                );
            }
        }

        Ok(())
    }

    fn canonical_payload_unchecked(&self) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(b"BMCOMPOSE\0");
        payload.push(1);
        payload.extend_from_slice(self.decomposition_digest.as_bytes());

        let mut component_values = self.component_values.iter().collect::<Vec<_>>();
        component_values
            .sort_by_key(|component| (component.component_root, component.value_digest.as_str()));
        let component_count =
            u16::try_from(component_values.len()).expect("too many component values");
        payload.extend_from_slice(&component_count.to_le_bytes());
        for component in component_values {
            payload.push(component.component_root);
            push_len_prefixed_bytes(&mut payload, component.value_digest.as_bytes());
        }
        push_len_prefixed_bytes(&mut payload, self.result_value_digest.as_bytes());
        payload
    }
}

impl DecompositionCertificate {
    /// Returns a versioned canonical byte payload for stable provenance labels.
    ///
    /// Components are serialized in sorted order, making the payload independent
    /// of the caller's component vector order. Validation runs before
    /// serialization. The payload records a board-graph certificate.
    pub fn canonical_payload(&self) -> Result<Vec<u8>, DecompositionCertificateValidationError> {
        self.validate()?;
        Ok(self.canonical_payload_unchecked())
    }

    /// Returns a stable SHA-256 digest of this certificate's canonical payload.
    ///
    /// Validation is run before hashing. The digest is deterministic across
    /// process runs for the same validated structural certificate.
    pub fn digest(
        &self,
    ) -> Result<DecompositionCertificateDigest, DecompositionCertificateValidationError> {
        Ok(DecompositionCertificateDigest(sha256(
            &self.canonical_payload()?,
        )))
    }

    /// Validates bounded structural invariants for this certificate.
    ///
    /// This checks mask/status consistency and audits 8-way adjacency between
    /// certified component masks. The validated claim is limited to that
    /// board-graph structure.
    pub fn validate(&self) -> Result<(), DecompositionCertificateValidationError> {
        let expected_strict = self.status == DecompositionStatus::Strict;
        if self.strict != expected_strict {
            return Err(
                DecompositionCertificateValidationError::StrictStatusMismatch {
                    strict: self.strict,
                    status: self.status,
                },
            );
        }

        if usize::from(self.active_component_count) != self.components.len() {
            return Err(
                DecompositionCertificateValidationError::ActiveComponentCountMismatch {
                    declared: self.active_component_count,
                    actual: self.components.len(),
                },
            );
        }

        match self.status {
            DecompositionStatus::Strict => {
                if self.components.len() < 2 {
                    return Err(
                        DecompositionCertificateValidationError::StrictWithTooFewActiveComponents {
                            active_component_count: self.components.len(),
                        },
                    );
                }
                if self.barrier.is_empty() {
                    return Err(DecompositionCertificateValidationError::StrictWithoutBarrier);
                }
                if let Some(rejection_reason) = self.rejection_reason {
                    return Err(
                        DecompositionCertificateValidationError::StrictWithRejectionReason {
                            rejection_reason,
                        },
                    );
                }
            }
            DecompositionStatus::Rejected => {
                use DecompositionCertificateValidationError as Error;

                match self.rejection_reason {
                    Some(DecompositionRejectionReason::NoLockedBarrier) => {
                        if !self.barrier.is_empty() {
                            return Err(Error::NoLockedBarrierRejectionWithBarrier);
                        }
                        if self.components.len() > 1 {
                            return Err(
                                Error::NoLockedBarrierRejectionWithMultipleActiveComponents {
                                    active_component_count: self.components.len(),
                                },
                            );
                        }
                    }
                    Some(DecompositionRejectionReason::LessThanTwoActiveComponents) => {
                        if self.barrier.is_empty() {
                            return Err(Error::LessThanTwoActiveComponentsRejectionWithoutBarrier);
                        }
                        if self.components.len() >= 2 {
                            return Err(
                                Error::LessThanTwoActiveComponentsRejectionWithTooManyActiveComponents {
                                    active_component_count: self.components.len(),
                                },
                            );
                        }
                    }
                    None => {
                        return Err(Error::RejectedWithoutRejectionReason);
                    }
                }
            }
        }

        let mut component_by_square = [None; 64];
        let mut component_by_root = [None; 64];
        for (component_index, component) in self.components.iter().enumerate() {
            if component.mask.is_empty() {
                return Err(
                    DecompositionCertificateValidationError::EmptyComponentMask { component_index },
                );
            }

            if component.active_mask.is_empty() {
                return Err(
                    DecompositionCertificateValidationError::ComponentWithoutActiveSquares {
                        component_index,
                    },
                );
            }

            if !component.mask.is_disjoint(self.barrier) {
                return Err(
                    DecompositionCertificateValidationError::ComponentIntersectsBarrier {
                        component_index,
                    },
                );
            }

            if !component.active_mask.is_subset(component.mask) {
                return Err(
                    DecompositionCertificateValidationError::ActiveMaskOutsideComponent {
                        component_index,
                    },
                );
            }

            let root_bit = 1u64.checked_shl(u32::from(component.root)).unwrap_or(0);
            if bitboard_bits(component.mask) & root_bit == 0 {
                return Err(
                    DecompositionCertificateValidationError::ComponentRootOutsideMask {
                        component_index,
                        root: component.root,
                    },
                );
            }

            let root_index = usize::from(component.root);
            if let Some(first_component_index) = component_by_root[root_index] {
                return Err(
                    DecompositionCertificateValidationError::DuplicateComponentRoot {
                        first_component_index,
                        second_component_index: component_index,
                        root: component.root,
                    },
                );
            }
            component_by_root[root_index] = Some(component_index);

            for sq in component.mask {
                let square_index = usize::from(sq);
                if let Some(first_component_index) = component_by_square[square_index] {
                    return Err(
                        DecompositionCertificateValidationError::ComponentMasksOverlap {
                            first_component_index,
                            second_component_index: component_index,
                        },
                    );
                }
                component_by_square[square_index] = Some(component_index);
            }
        }

        for (component_index, component) in self.components.iter().enumerate() {
            for sq in component.mask {
                for (file_delta, rank_delta) in EIGHT_WAY_DELTAS {
                    if let Some(adjacent) = adjacent_square(sq, file_delta, rank_delta) {
                        let adjacent_index = usize::from(adjacent);
                        match component_by_square[adjacent_index] {
                            Some(adjacent_component_index) => {
                                if adjacent_component_index != component_index {
                                    return Err(
                                        DecompositionCertificateValidationError::CrossComponentAdjacency {
                                            first_component_index: component_index,
                                            second_component_index: adjacent_component_index,
                                            first_square: sq,
                                            second_square: adjacent,
                                        },
                                    );
                                }
                            }
                            None => {
                                if self.status == DecompositionStatus::Strict
                                    && !self.barrier.contains(adjacent)
                                {
                                    return Err(
                                        DecompositionCertificateValidationError::StrictComponentMaskNotClosed {
                                            component_index,
                                            square: sq,
                                            omitted_square: adjacent,
                                        },
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn canonical_payload_unchecked(&self) -> Vec<u8> {
        let mut payload = Vec::with_capacity(22 + self.components.len() * 17);
        payload.extend_from_slice(b"BMDCERT\0");
        payload.push(1);
        payload.push(decomposition_status_tag(self.status));
        payload.push(u8::from(self.strict));
        payload.push(decomposition_rejection_reason_tag(self.rejection_reason));
        payload.push(self.active_component_count);
        payload.extend_from_slice(&bitboard_bits(self.barrier).to_le_bytes());
        payload.push(self.components.len() as u8);

        let mut components: Vec<_> = self.components.iter().collect();
        components.sort_by_key(|component| {
            (
                component.root,
                bitboard_bits(component.mask),
                bitboard_bits(component.active_mask),
            )
        });

        for component in components {
            payload.push(component.root);
            payload.extend_from_slice(&bitboard_bits(component.mask).to_le_bytes());
            payload.extend_from_slice(&bitboard_bits(component.active_mask).to_le_bytes());
        }

        payload
    }
}
