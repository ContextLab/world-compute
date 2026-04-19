//! Shard placement and geographic diversity enforcement per FR-074.

use crate::error::{ErrorCode, WcError};
use std::collections::HashMap;

/// Configuration for shard placement constraints.
#[derive(Debug, Clone)]
pub struct PlacementConfig {
    /// Minimum number of distinct continents across all placements.
    pub min_continents: usize,
    /// Maximum number of placements allowed in any single country.
    pub max_per_country: usize,
    /// Minimum number of distinct autonomous systems required.
    pub min_per_as: usize,
}

impl Default for PlacementConfig {
    fn default() -> Self {
        Self { min_continents: 3, max_per_country: 2, min_per_as: 1 }
    }
}

/// A single shard-to-node placement record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShardPlacement {
    pub shard_id: u32,
    pub node_id: String,
    pub continent: String,
    pub country: String,
    pub autonomous_system: u32,
}

/// Validate that a set of shard placements satisfies geographic diversity
/// constraints from `config`.
///
/// Checks:
/// 1. At least `min_continents` distinct continents are covered.
/// 2. No single country has more than `max_per_country` placements.
/// 3. At least `min_per_as` distinct autonomous systems are present.
pub fn validate_placement(
    placements: &[ShardPlacement],
    config: &PlacementConfig,
) -> Result<(), WcError> {
    if placements.is_empty() {
        return Ok(());
    }

    // Count distinct continents
    let continents: std::collections::HashSet<&str> =
        placements.iter().map(|p| p.continent.as_str()).collect();
    if continents.len() < config.min_continents {
        return Err(WcError::new(
            ErrorCode::ResidencyConstraintViolation,
            format!(
                "Placement spans only {} continent(s); minimum is {}",
                continents.len(),
                config.min_continents
            ),
        ));
    }

    // Count placements per country
    let mut per_country: HashMap<&str, usize> = HashMap::new();
    for p in placements {
        *per_country.entry(p.country.as_str()).or_insert(0) += 1;
    }
    for (country, count) in &per_country {
        if *count > config.max_per_country {
            return Err(WcError::new(
                ErrorCode::ResidencyConstraintViolation,
                format!(
                    "Country '{}' has {} placements; maximum is {}",
                    country, count, config.max_per_country
                ),
            ));
        }
    }

    // Count distinct autonomous systems
    let as_set: std::collections::HashSet<u32> =
        placements.iter().map(|p| p.autonomous_system).collect();
    if as_set.len() < config.min_per_as {
        return Err(WcError::new(
            ErrorCode::ResidencyConstraintViolation,
            format!(
                "Placement uses only {} AS(es); minimum is {}",
                as_set.len(),
                config.min_per_as
            ),
        ));
    }

    Ok(())
}

/// Check whether a node's jurisdiction matches the required data residency.
///
/// Returns `true` if the node is in the correct jurisdiction for the data,
/// or if the data residency requirement is "any" (no restriction).
pub fn check_shard_residency(node_jurisdiction: &str, data_residency: &str) -> bool {
    if data_residency.eq_ignore_ascii_case("any") || data_residency.is_empty() {
        return true;
    }
    node_jurisdiction.eq_ignore_ascii_case(data_residency)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diverse_placements() -> Vec<ShardPlacement> {
        vec![
            ShardPlacement {
                shard_id: 0,
                node_id: "node-us".into(),
                continent: "NA".into(),
                country: "US".into(),
                autonomous_system: 15169,
            },
            ShardPlacement {
                shard_id: 1,
                node_id: "node-de".into(),
                continent: "EU".into(),
                country: "DE".into(),
                autonomous_system: 3320,
            },
            ShardPlacement {
                shard_id: 2,
                node_id: "node-jp".into(),
                continent: "AS".into(),
                country: "JP".into(),
                autonomous_system: 2497,
            },
        ]
    }

    #[test]
    fn valid_diverse_placement_passes() {
        let result = validate_placement(&diverse_placements(), &PlacementConfig::default());
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn all_same_country_fails() {
        let placements = vec![
            ShardPlacement {
                shard_id: 0,
                node_id: "n1".into(),
                continent: "NA".into(),
                country: "US".into(),
                autonomous_system: 15169,
            },
            ShardPlacement {
                shard_id: 1,
                node_id: "n2".into(),
                continent: "NA".into(),
                country: "US".into(),
                autonomous_system: 7922,
            },
            ShardPlacement {
                shard_id: 2,
                node_id: "n3".into(),
                continent: "NA".into(),
                country: "US".into(),
                autonomous_system: 701,
            },
        ];
        let result = validate_placement(&placements, &PlacementConfig::default());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), Some(ErrorCode::ResidencyConstraintViolation));
    }

    #[test]
    fn too_few_continents_fails() {
        let placements = vec![
            ShardPlacement {
                shard_id: 0,
                node_id: "n1".into(),
                continent: "EU".into(),
                country: "DE".into(),
                autonomous_system: 3320,
            },
            ShardPlacement {
                shard_id: 1,
                node_id: "n2".into(),
                continent: "EU".into(),
                country: "FR".into(),
                autonomous_system: 5410,
            },
        ];
        let result = validate_placement(&placements, &PlacementConfig::default());
        assert!(result.is_err());
    }

    #[test]
    fn empty_placements_pass() {
        let result = validate_placement(&[], &PlacementConfig::default());
        assert!(result.is_ok());
    }
}
