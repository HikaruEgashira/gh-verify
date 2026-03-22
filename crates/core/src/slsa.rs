//! SLSA v1.2 track and level definitions.
//!
//! Defines the two-track model (Source + Build) with progressive levels,
//! and maps each [`ControlId`] to its SLSA track and minimum level.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::control::ControlId;

/// SLSA specification tracks per v1.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlsaTrack {
    /// Source code integrity and review controls.
    Source,
    /// Build process integrity and provenance controls.
    Build,
}

impl fmt::Display for SlsaTrack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source => f.write_str("source"),
            Self::Build => f.write_str("build"),
        }
    }
}

/// SLSA levels within a track (v1.2).
///
/// Levels are cumulative: achieving L3 implies L2 and L1 are also met.
/// L4 is defined only for the Source track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlsaLevel {
    /// No SLSA requirements met.
    L0,
    /// Source L1: Version controlled. Build L1: Provenance exists.
    L1,
    /// Source L2: History & provenance. Build L2: Hosted + authenticated provenance.
    L2,
    /// Source L3: Continuous technical controls. Build L3: Hardened builds.
    L3,
    /// Source L4: Two-party review. (Build track does not define L4.)
    L4,
}

impl SlsaLevel {
    /// Returns true if this level is valid for the given track.
    pub fn is_valid_for_track(self, track: SlsaTrack) -> bool {
        match track {
            SlsaTrack::Source => true,                 // Source track defines L0-L4
            SlsaTrack::Build => self <= SlsaLevel::L3, // Build track defines L0-L3
        }
    }
}

impl fmt::Display for SlsaLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::L0 => f.write_str("L0"),
            Self::L1 => f.write_str("L1"),
            Self::L2 => f.write_str("L2"),
            Self::L3 => f.write_str("L3"),
            Self::L4 => f.write_str("L4"),
        }
    }
}

/// Mapping of a control to its SLSA track and the minimum level it satisfies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlsaMapping {
    pub track: SlsaTrack,
    pub level: SlsaLevel,
}

/// Returns the SLSA track and minimum level for a control.
///
/// Controls not part of the SLSA framework (dev quality controls) return `None`.
pub fn control_slsa_mapping(id: ControlId) -> Option<SlsaMapping> {
    match id {
        // Source Track
        ControlId::SourceAuthenticity => Some(SlsaMapping {
            track: SlsaTrack::Source,
            level: SlsaLevel::L1,
        }),
        ControlId::ReviewIndependence => Some(SlsaMapping {
            track: SlsaTrack::Source,
            level: SlsaLevel::L1,
        }),
        ControlId::BranchHistoryIntegrity => Some(SlsaMapping {
            track: SlsaTrack::Source,
            level: SlsaLevel::L2,
        }),
        ControlId::BranchProtectionEnforcement => Some(SlsaMapping {
            track: SlsaTrack::Source,
            level: SlsaLevel::L3,
        }),
        ControlId::TwoPartyReview => Some(SlsaMapping {
            track: SlsaTrack::Source,
            level: SlsaLevel::L4,
        }),

        // Build Track
        ControlId::BuildProvenance => Some(SlsaMapping {
            track: SlsaTrack::Build,
            level: SlsaLevel::L1,
        }),
        ControlId::RequiredStatusChecks => Some(SlsaMapping {
            track: SlsaTrack::Build,
            level: SlsaLevel::L1,
        }),
        ControlId::HostedBuildPlatform => Some(SlsaMapping {
            track: SlsaTrack::Build,
            level: SlsaLevel::L2,
        }),
        ControlId::ProvenanceAuthenticity => Some(SlsaMapping {
            track: SlsaTrack::Build,
            level: SlsaLevel::L2,
        }),
        ControlId::BuildIsolation => Some(SlsaMapping {
            track: SlsaTrack::Build,
            level: SlsaLevel::L3,
        }),

        // Dev quality controls are not SLSA-mapped
        ControlId::PrSize
        | ControlId::TestCoverage
        | ControlId::ScopedChange
        | ControlId::IssueLinkage => None,
    }
}

/// Returns all controls required at or below the given level for a track.
pub fn controls_for_level(track: SlsaTrack, level: SlsaLevel) -> Vec<ControlId> {
    ALL_SLSA_CONTROLS
        .iter()
        .copied()
        .filter(|&id| {
            control_slsa_mapping(id).is_some_and(|m| m.track == track && m.level <= level)
        })
        .collect()
}

/// All SLSA-mapped control IDs in declaration order.
const ALL_SLSA_CONTROLS: &[ControlId] = &[
    // Source track
    ControlId::SourceAuthenticity,
    ControlId::ReviewIndependence,
    ControlId::BranchHistoryIntegrity,
    ControlId::BranchProtectionEnforcement,
    ControlId::TwoPartyReview,
    // Build track
    ControlId::BuildProvenance,
    ControlId::RequiredStatusChecks,
    ControlId::HostedBuildPlatform,
    ControlId::ProvenanceAuthenticity,
    ControlId::BuildIsolation,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_l1_controls() {
        let controls = controls_for_level(SlsaTrack::Source, SlsaLevel::L1);
        assert!(controls.contains(&ControlId::SourceAuthenticity));
        assert!(controls.contains(&ControlId::ReviewIndependence));
        assert!(!controls.contains(&ControlId::BranchHistoryIntegrity));
    }

    #[test]
    fn source_l4_includes_all_source_controls() {
        let controls = controls_for_level(SlsaTrack::Source, SlsaLevel::L4);
        assert_eq!(controls.len(), 5);
        assert!(controls.contains(&ControlId::TwoPartyReview));
    }

    #[test]
    fn build_l3_includes_all_build_controls() {
        let controls = controls_for_level(SlsaTrack::Build, SlsaLevel::L3);
        assert_eq!(controls.len(), 5);
        assert!(controls.contains(&ControlId::BuildIsolation));
    }

    #[test]
    fn dev_quality_controls_have_no_slsa_mapping() {
        assert!(control_slsa_mapping(ControlId::PrSize).is_none());
        assert!(control_slsa_mapping(ControlId::TestCoverage).is_none());
        assert!(control_slsa_mapping(ControlId::ScopedChange).is_none());
        assert!(control_slsa_mapping(ControlId::IssueLinkage).is_none());
    }

    #[test]
    fn l4_not_valid_for_build_track() {
        assert!(!SlsaLevel::L4.is_valid_for_track(SlsaTrack::Build));
        assert!(SlsaLevel::L4.is_valid_for_track(SlsaTrack::Source));
    }

    #[test]
    fn levels_are_ordered() {
        assert!(SlsaLevel::L0 < SlsaLevel::L1);
        assert!(SlsaLevel::L1 < SlsaLevel::L2);
        assert!(SlsaLevel::L2 < SlsaLevel::L3);
        assert!(SlsaLevel::L3 < SlsaLevel::L4);
    }

    #[test]
    fn l0_returns_empty() {
        assert!(controls_for_level(SlsaTrack::Source, SlsaLevel::L0).is_empty());
        assert!(controls_for_level(SlsaTrack::Build, SlsaLevel::L0).is_empty());
    }
}
