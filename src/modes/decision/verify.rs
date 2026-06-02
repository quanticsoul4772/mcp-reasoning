//! Arithmetic verification for decision analysis.
//!
//! The model is asked to compute weighted totals, TOPSIS closeness, pairwise
//! win counts, and the rankings that follow from them. Language models slip on
//! this arithmetic, so every operation recomputes the numbers from the
//! underlying scores, corrects the ranking when it disagrees, and records any
//! discrepancy in a [`DecisionValidation`].

use std::cmp::Ordering;
use std::collections::HashMap;

use super::types::{
    Criterion, DecisionValidation, PairwiseComparison, PairwiseRank, PreferenceResult,
    RankedOption, TopsisCreterion, TopsisDistances, TopsisRank,
};

/// Absolute tolerance for treating two computed values as equal.
const TOLERANCE: f64 = 0.01;
/// Margin below which the top two options are reported as a near-tie.
const NEAR_TIE_MARGIN: f64 = 0.05;

/// Compare two scores descending, breaking ties by option name for determinism.
fn by_score_desc(a: &(String, f64), b: &(String, f64)) -> Ordering {
    b.1.partial_cmp(&a.1)
        .unwrap_or(Ordering::Equal)
        .then_with(|| a.0.cmp(&b.0))
}

/// Verify weighted multi-criteria arithmetic.
///
/// Recomputes each option's total as `Σ weightᵢ · scoreᵢ`, adopts the
/// recomputed totals, and re-derives the ranking. `weighted_totals` and
/// `ranking` are corrected in place.
pub(super) fn verify_weighted(
    criteria: &[Criterion],
    scores: &HashMap<String, HashMap<String, f64>>,
    weighted_totals: &mut HashMap<String, f64>,
    ranking: &mut Vec<RankedOption>,
) -> DecisionValidation {
    let mut validation = DecisionValidation::default();

    let weight_sum: f64 = criteria.iter().map(|c| c.weight).sum();
    if (weight_sum - 1.0).abs() > TOLERANCE {
        validation.consistent = false;
        validation
            .warnings
            .push(format!("Criteria weights sum to {weight_sum:.3}, not 1.0"));
    }

    // Recompute each option's weighted total from its per-criterion scores.
    let mut recomputed: HashMap<String, f64> = HashMap::new();
    for (option, option_scores) in scores {
        let total: f64 = criteria
            .iter()
            .map(|c| c.weight * option_scores.get(&c.name).copied().unwrap_or(0.0))
            .sum();
        if let Some(&stated) = weighted_totals.get(option) {
            if (stated - total).abs() > TOLERANCE {
                validation.consistent = false;
                validation.warnings.push(format!(
                    "Weighted total for '{option}' was stated as {stated:.3} but recomputes to {total:.3}"
                ));
            }
        }
        recomputed.insert(option.clone(), total);
    }

    *weighted_totals = recomputed.clone();

    let new_ranking = rank_from_scores(recomputed, |option, score, rank| RankedOption {
        option,
        score,
        rank,
    });

    if !same_order(
        ranking.iter().map(|r| &r.option),
        new_ranking.iter().map(|r| &r.option),
    ) {
        validation.consistent = false;
        validation.ranking_corrected = true;
        validation
            .warnings
            .push("Ranking re-derived from verified weighted totals.".to_string());
    }
    *ranking = new_ranking;

    validation
}

/// Verify TOPSIS closeness and ranking.
///
/// Recomputes relative closeness as `d⁻ / (d⁺ + d⁻)`, adopts the recomputed
/// values, and re-derives the ranking. `relative_closeness` and `ranking` are
/// corrected in place.
pub(super) fn verify_topsis(
    criteria: &[TopsisCreterion],
    distances: &HashMap<String, TopsisDistances>,
    relative_closeness: &mut HashMap<String, f64>,
    ranking: &mut Vec<TopsisRank>,
) -> DecisionValidation {
    let mut validation = DecisionValidation::default();

    let weight_sum: f64 = criteria.iter().map(|c| c.weight).sum();
    if (weight_sum - 1.0).abs() > TOLERANCE {
        validation.consistent = false;
        validation
            .warnings
            .push(format!("Criteria weights sum to {weight_sum:.3}, not 1.0"));
    }

    let mut recomputed: HashMap<String, f64> = HashMap::new();
    for (option, dist) in distances {
        let denom = dist.to_ideal + dist.to_anti_ideal;
        let closeness = if denom.abs() < f64::EPSILON {
            validation.consistent = false;
            validation.warnings.push(format!(
                "Distances for '{option}' sum to zero; closeness is undefined"
            ));
            0.0
        } else {
            dist.to_anti_ideal / denom
        };
        if let Some(&stated) = relative_closeness.get(option) {
            if (stated - closeness).abs() > TOLERANCE {
                validation.consistent = false;
                validation.warnings.push(format!(
                    "Closeness for '{option}' was stated as {stated:.3} but recomputes to {closeness:.3}"
                ));
            }
        }
        recomputed.insert(option.clone(), closeness);
    }

    *relative_closeness = recomputed.clone();

    let new_ranking = rank_from_scores(recomputed, |option, closeness, rank| TopsisRank {
        option,
        closeness,
        rank,
    });

    if !same_order(
        ranking.iter().map(|r| &r.option),
        new_ranking.iter().map(|r| &r.option),
    ) {
        validation.consistent = false;
        validation.ranking_corrected = true;
        validation
            .warnings
            .push("Ranking re-derived from verified closeness values.".to_string());
    }
    *ranking = new_ranking;

    validation
}

/// Verify pairwise win counts, ranking, and transitivity.
///
/// Recounts wins directly from the comparisons and re-derives the ranking.
/// `ranking` is corrected in place.
pub(super) fn verify_pairwise(
    comparisons: &[PairwiseComparison],
    ranking: &mut Vec<PairwiseRank>,
) -> DecisionValidation {
    let mut validation = DecisionValidation::default();

    // Recount wins from the comparisons themselves.
    let mut wins: HashMap<String, u32> = HashMap::new();
    for cmp in comparisons {
        wins.entry(cmp.option_a.clone()).or_insert(0);
        wins.entry(cmp.option_b.clone()).or_insert(0);
        match cmp.preferred {
            PreferenceResult::OptionA => *wins.entry(cmp.option_a.clone()).or_insert(0) += 1,
            PreferenceResult::OptionB => *wins.entry(cmp.option_b.clone()).or_insert(0) += 1,
            PreferenceResult::Tie => {}
        }
    }

    // Flag stated win counts that disagree with the recount.
    for rank in ranking.iter() {
        if let Some(&recounted) = wins.get(&rank.option) {
            if rank.wins != recounted {
                validation.consistent = false;
                validation.warnings.push(format!(
                    "Win count for '{}' was stated as {} but recounts to {recounted}",
                    rank.option, rank.wins
                ));
            }
        }
    }

    if let Some(cycle) = find_intransitivity(comparisons) {
        validation.consistent = false;
        validation.warnings.push(format!(
            "Intransitive preferences detected: {} > {} > {} > {}",
            cycle.0, cycle.1, cycle.2, cycle.0
        ));
    }

    let win_pairs: HashMap<String, f64> = wins
        .iter()
        .map(|(o, w)| (o.clone(), f64::from(*w)))
        .collect();
    let new_ranking = rank_from_scores(win_pairs, |option, score, rank| PairwiseRank {
        option,
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        wins: score as u32,
        rank,
    });

    if !same_order(
        ranking.iter().map(|r| &r.option),
        new_ranking.iter().map(|r| &r.option),
    ) {
        validation.consistent = false;
        validation.ranking_corrected = true;
        validation
            .warnings
            .push("Ranking re-derived from recounted pairwise wins.".to_string());
    }
    *ranking = new_ranking;

    validation
}

/// Build a human-readable rationale describing the TOPSIS closeness spread.
pub(super) fn topsis_rationale(ranking: &[TopsisRank]) -> String {
    match ranking {
        [] => "No options were ranked.".to_string(),
        [only] => format!(
            "Only one option ('{}', closeness {:.3}); no comparison possible.",
            only.option, only.closeness
        ),
        [top, runner_up, ..] => {
            let margin = top.closeness - runner_up.closeness;
            let base = format!(
                "'{}' leads with closeness {:.3}, ahead of '{}' ({:.3}) by {:.3}",
                top.option, top.closeness, runner_up.option, runner_up.closeness, margin
            );
            if margin < NEAR_TIE_MARGIN {
                format!(
                    "{base}. Near-tie (margin < {NEAR_TIE_MARGIN}): consider additional criteria to separate the top options."
                )
            } else {
                format!("{base}.")
            }
        }
    }
}

/// Sort `scores` descending and assign sequential ranks (1 = best), breaking
/// ties by option name so ranks are always dense and deterministic.
fn rank_from_scores<T>(
    scores: HashMap<String, f64>,
    make: impl Fn(String, f64, u32) -> T,
) -> Vec<T> {
    let mut ordered: Vec<(String, f64)> = scores.into_iter().collect();
    ordered.sort_by(by_score_desc);
    ordered
        .into_iter()
        .enumerate()
        .map(|(idx, (option, score))| {
            #[allow(clippy::cast_possible_truncation)]
            make(option, score, (idx + 1) as u32)
        })
        .collect()
}

/// True when two option sequences list the same options in the same order.
fn same_order<'a>(
    a: impl Iterator<Item = &'a String>,
    b: impl Iterator<Item = &'a String>,
) -> bool {
    a.eq(b)
}

/// Find a 3-cycle (a beats b, b beats c, c beats a) in the strict-preference
/// relation, if any. Returns the cycle as `(a, b, c)`.
fn find_intransitivity(comparisons: &[PairwiseComparison]) -> Option<(String, String, String)> {
    // Set of (winner, loser) strict preferences.
    let mut beats: Vec<(String, String)> = Vec::new();
    for cmp in comparisons {
        match cmp.preferred {
            PreferenceResult::OptionA => beats.push((cmp.option_a.clone(), cmp.option_b.clone())),
            PreferenceResult::OptionB => beats.push((cmp.option_b.clone(), cmp.option_a.clone())),
            PreferenceResult::Tie => {}
        }
    }
    let beats_set: std::collections::HashSet<(String, String)> = beats.iter().cloned().collect();
    for (a, b) in &beats {
        for (b2, c) in &beats {
            if b == b2 && a != c && beats_set.contains(&(c.clone(), a.clone())) {
                return Some((a.clone(), b.clone(), c.clone()));
            }
        }
    }
    None
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::panic
)]
mod tests {
    use super::super::types::{CriterionType, PreferenceStrength};
    use super::*;

    fn crit(name: &str, weight: f64) -> Criterion {
        Criterion {
            name: name.to_string(),
            weight,
            description: String::new(),
        }
    }

    #[test]
    fn weighted_recomputes_totals_and_corrects_ranking() {
        let criteria = vec![crit("Cost", 0.5), crit("Speed", 0.5)];
        let mut scores: HashMap<String, HashMap<String, f64>> = HashMap::new();
        scores.insert(
            "A".to_string(),
            HashMap::from([("Cost".to_string(), 0.2), ("Speed".to_string(), 0.2)]),
        );
        scores.insert(
            "B".to_string(),
            HashMap::from([("Cost".to_string(), 0.9), ("Speed".to_string(), 0.9)]),
        );
        // Model wrongly claims A wins with an inflated total.
        let mut totals = HashMap::from([("A".to_string(), 0.95), ("B".to_string(), 0.9)]);
        let mut ranking = vec![
            RankedOption {
                option: "A".to_string(),
                score: 0.95,
                rank: 1,
            },
            RankedOption {
                option: "B".to_string(),
                score: 0.9,
                rank: 2,
            },
        ];

        let v = verify_weighted(&criteria, &scores, &mut totals, &mut ranking);

        assert!(!v.consistent);
        assert!(v.ranking_corrected);
        assert_eq!(ranking[0].option, "B");
        assert!((totals["A"] - 0.2).abs() < 1e-9);
        assert!((totals["B"] - 0.9).abs() < 1e-9);
    }

    #[test]
    fn weighted_consistent_when_arithmetic_matches() {
        let criteria = vec![crit("Cost", 0.5), crit("Speed", 0.5)];
        let scores = HashMap::from([(
            "A".to_string(),
            HashMap::from([("Cost".to_string(), 0.6), ("Speed".to_string(), 0.8)]),
        )]);
        let mut totals = HashMap::from([("A".to_string(), 0.7)]);
        let mut ranking = vec![RankedOption {
            option: "A".to_string(),
            score: 0.7,
            rank: 1,
        }];

        let v = verify_weighted(&criteria, &scores, &mut totals, &mut ranking);
        assert!(v.consistent);
        assert!(!v.ranking_corrected);
        assert!(v.warnings.is_empty());
    }

    #[test]
    fn weighted_flags_bad_weight_sum() {
        let criteria = vec![crit("Cost", 0.5), crit("Speed", 0.3)];
        let scores = HashMap::from([(
            "A".to_string(),
            HashMap::from([("Cost".to_string(), 0.5), ("Speed".to_string(), 0.5)]),
        )]);
        let mut totals = HashMap::from([("A".to_string(), 0.4)]);
        let mut ranking = vec![RankedOption {
            option: "A".to_string(),
            score: 0.4,
            rank: 1,
        }];

        let v = verify_weighted(&criteria, &scores, &mut totals, &mut ranking);
        assert!(!v.consistent);
        assert!(v.warnings.iter().any(|w| w.contains("sum to 0.800")));
    }

    #[test]
    fn topsis_recomputes_closeness() {
        let criteria = vec![TopsisCreterion {
            name: "C".to_string(),
            criterion_type: CriterionType::Benefit,
            weight: 1.0,
        }];
        let distances = HashMap::from([
            (
                "A".to_string(),
                TopsisDistances {
                    to_ideal: 0.2,
                    to_anti_ideal: 0.8,
                },
            ),
            (
                "B".to_string(),
                TopsisDistances {
                    to_ideal: 0.8,
                    to_anti_ideal: 0.2,
                },
            ),
        ]);
        // Model wrongly reports A behind B.
        let mut closeness = HashMap::from([("A".to_string(), 0.3), ("B".to_string(), 0.7)]);
        let mut ranking = vec![
            TopsisRank {
                option: "B".to_string(),
                closeness: 0.7,
                rank: 1,
            },
            TopsisRank {
                option: "A".to_string(),
                closeness: 0.3,
                rank: 2,
            },
        ];

        let v = verify_topsis(&criteria, &distances, &mut closeness, &mut ranking);
        assert!(!v.consistent);
        assert!(v.ranking_corrected);
        assert_eq!(ranking[0].option, "A");
        assert!((closeness["A"] - 0.8).abs() < 1e-9);
    }

    #[test]
    fn pairwise_recounts_wins_and_detects_cycle() {
        let cmp = |a: &str, b: &str, pref: PreferenceResult| PairwiseComparison {
            option_a: a.to_string(),
            option_b: b.to_string(),
            preferred: pref,
            strength: PreferenceStrength::Moderate,
            reasoning: String::new(),
        };
        let comparisons = vec![
            cmp("A", "B", PreferenceResult::OptionA),
            cmp("B", "C", PreferenceResult::OptionA),
            cmp("A", "C", PreferenceResult::OptionB),
        ];
        let mut ranking = vec![
            PairwiseRank {
                option: "A".to_string(),
                wins: 5,
                rank: 1,
            },
            PairwiseRank {
                option: "B".to_string(),
                wins: 1,
                rank: 2,
            },
            PairwiseRank {
                option: "C".to_string(),
                wins: 1,
                rank: 3,
            },
        ];

        let v = verify_pairwise(&comparisons, &mut ranking);
        assert!(!v.consistent);
        // A wins A>B, B wins B>C, C wins A-vs-C → A:1, B:1, C:1, intransitive.
        assert!(v.warnings.iter().any(|w| w.contains("Intransitive")));
        assert!(v.warnings.iter().any(|w| w.contains("Win count for 'A'")));
    }

    #[test]
    fn topsis_rationale_flags_near_tie() {
        let ranking = vec![
            TopsisRank {
                option: "A".to_string(),
                closeness: 0.62,
                rank: 1,
            },
            TopsisRank {
                option: "B".to_string(),
                closeness: 0.60,
                rank: 2,
            },
        ];
        let r = topsis_rationale(&ranking);
        assert!(r.contains("Near-tie"));
    }

    #[test]
    fn topsis_rationale_clear_winner() {
        let ranking = vec![
            TopsisRank {
                option: "A".to_string(),
                closeness: 0.9,
                rank: 1,
            },
            TopsisRank {
                option: "B".to_string(),
                closeness: 0.4,
                rank: 2,
            },
        ];
        let r = topsis_rationale(&ranking);
        assert!(!r.contains("Near-tie"));
        assert!(r.contains("leads"));
    }
}
