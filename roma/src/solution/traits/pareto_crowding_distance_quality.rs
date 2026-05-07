/// Multi-objective quality metadata based on Pareto rank and crowding distance.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParetoCrowdingDistanceQuality {
    /// Objective values in minimization form.
    pub objectives: Vec<f64>,
    /// Non-dominated sorting rank (lower is better).
    pub rank: Option<usize>,
    /// Crowding distance used as tie-breaker within same rank.
    pub crowding_distance: Option<f64>,
}

impl ParetoCrowdingDistanceQuality {
    /// Pareto dominance based on objective vectors (minimization semantics).
    ///
    /// Rules:
    /// - `self` dominates `other` iff all objectives are <= and at least one
    ///   objective is strictly <,
    /// - rank and crowding-distance are **not** part of Pareto dominance;
    ///   they are selection metadata used as tie-breakers in algorithms
    ///   (e.g., NSGA-II tournament/replacement).
    /// - if objective vectors are missing, empty, or mismatched in length,
    ///   dominance is undefined and this returns `false`.
    ///
    /// In Pareto optimization, it is common that two solutions are
    /// non-dominated with respect to each other. In that case this method must
    /// return `false` in both directions.
    fn dominates(&self, other: &Self) -> bool {
        if self.objectives.is_empty()
            || other.objectives.is_empty()
            || self.objectives.len() != other.objectives.len()
        {
            return false;
        }

        let mut strictly_better_in_any = false;
        for (&a, &b) in self.objectives.iter().zip(other.objectives.iter()) {
            if a > b {
                return false;
            }
            if a < b {
                strictly_better_in_any = true;
            }
        }

        strictly_better_in_any
    }
}

use std::fmt;

impl Solution<f64, ParetoCrowdingDistanceQuality> {
    /// Returns the first objective if present.
    pub fn objective(&self) -> Option<f64> {
        self.value
            .as_ref()
            .and_then(|info| info.objectives.first().copied())
    }

    pub fn dominates(&self, other: &Self) -> bool {
        match (&self.value, &other.value) {
            (Some(a), Some(b)) => a.dominates(b),
            _ => false,
        }
    }
}

impl fmt::Display for ParetoCrowdingDistanceQuality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let objectives_fmt: String = self
            .objectives
            .iter()
            .map(|obj| format!("{:.4}", obj))
            .collect::<Vec<_>>()
            .join(", ");

        let rank_fmt = self
            .rank
            .map(|r| r.to_string())
            .unwrap_or_else(|| "N/A".to_string());

        let dist_fmt = self
            .crowding_distance
            .map(|d| format!("{:.4}", d))
            .unwrap_or_else(|| "N/A".to_string());

        write!(
            f,
            "Objectives: [{}], Rank: {}, Dist: {}",
            objectives_fmt, rank_fmt, dist_fmt
        )
    }
}

use std::str::FromStr;

use crate::Solution;

impl FromStr for ParetoCrowdingDistanceQuality {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Expected format: "Objectives: [0.1234, 0.5678], Rank: 1, Dist: 0.5000"

        // 1. Extract Objectives content between '[' and ']'
        let start_obj = s
            .find('[')
            .ok_or("Missing objectives opening bracket '['")?
            + 1;
        let end_obj = s
            .find(']')
            .ok_or("Missing objectives closing bracket ']'")?;
        let objectives_str = &s[start_obj..end_obj].trim();

        let objectives = if objectives_str.is_empty() {
            Vec::new()
        } else {
            objectives_str
                .split(", ")
                .map(|val| {
                    val.parse::<f64>()
                        .map_err(|_| format!("Failed to parse objective value: '{}'", val))
                })
                .collect::<Result<Vec<f64>, String>>()?
        };

        // 2. Parse Rank
        let rank_tag = "Rank: ";
        let start_rank = s.find(rank_tag).ok_or("Missing 'Rank:' identifier")? + rank_tag.len();

        // Find the comma after rank or take the rest of the string
        let end_rank = s[start_rank..]
            .find(',')
            .map(|i| i + start_rank)
            .unwrap_or(s.len());
        let rank_str = s[start_rank..end_rank].trim();

        let rank = if rank_str == "N/A" {
            None
        } else {
            Some(
                rank_str
                    .parse::<usize>()
                    .map_err(|_| "Invalid Rank format".to_string())?,
            )
        };

        // 3. Parse Crowding Distance
        let dist_tag = "Dist: ";
        let start_dist = s.find(dist_tag).ok_or("Missing 'Dist:' identifier")? + dist_tag.len();
        let dist_str = s[start_dist..].trim();

        let crowding_distance = if dist_str == "N/A" {
            None
        } else {
            Some(
                dist_str
                    .parse::<f64>()
                    .map_err(|_| "Invalid Distance format".to_string())?,
            )
        };

        Ok(ParetoCrowdingDistanceQuality {
            objectives,
            rank,
            crowding_distance,
        })
    }
}
