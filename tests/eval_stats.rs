//! Tests for the eval harness statistical primitives ([`mcp_reasoning::eval::stats`]).
//!
//! Closed-form and reference-value checks: every public function is verified
//! against a hand-computed expectation or a standard statistical identity, plus
//! Miller's conventional power constants (arXiv:2411.00640).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

use mcp_reasoning::eval::stats::{
    clears_mde, clustered_stderr, inverse_normal_cdf, lower_confidence_bound, mean_and_stderr,
    minimum_detectable_effect, paired_difference, required_n, Estimate,
};

/// Absolute-tolerance comparison for floating-point assertions.
fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() < tol
}

// ---- inverse_normal_cdf ----------------------------------------------------

#[test]
fn invcdf_known_critical_values() {
    // Standard reference z-values.
    assert!(close(
        inverse_normal_cdf(0.975).unwrap(),
        1.959_963_985,
        1e-6
    ));
    assert!(close(
        inverse_normal_cdf(0.95).unwrap(),
        1.644_853_627,
        1e-6
    ));
    assert!(close(
        inverse_normal_cdf(0.99).unwrap(),
        2.326_347_874,
        1e-6
    ));
    assert!(close(
        inverse_normal_cdf(0.80).unwrap(),
        0.841_621_234,
        1e-6
    ));
}

#[test]
fn invcdf_median_is_zero() {
    assert!(close(inverse_normal_cdf(0.5).unwrap(), 0.0, 1e-9));
}

#[test]
fn invcdf_is_antisymmetric() {
    // Φ⁻¹(p) = -Φ⁻¹(1 - p).
    for &p in &[0.01, 0.1, 0.3, 0.45] {
        let lo = inverse_normal_cdf(p).unwrap();
        let hi = inverse_normal_cdf(1.0 - p).unwrap();
        assert!(close(lo, -hi, 1e-6), "p={p}");
    }
}

#[test]
fn invcdf_rejects_out_of_range() {
    assert!(inverse_normal_cdf(0.0).is_none());
    assert!(inverse_normal_cdf(1.0).is_none());
    assert!(inverse_normal_cdf(-0.1).is_none());
    assert!(inverse_normal_cdf(1.1).is_none());
    assert!(inverse_normal_cdf(f64::NAN).is_none());
}

// ---- mean_and_stderr -------------------------------------------------------

#[test]
fn mean_stderr_closed_form() {
    // Sample: 2,4,4,4,5,5,7,9. mean=5. Σ(x-mean)² = 32, so the Bessel
    // (n-1) variance is 32/7 and SE = sqrt(32/7)/sqrt(8) = sqrt(32/56).
    let s = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
    let est = mean_and_stderr(&s).unwrap();
    assert!(close(est.mean, 5.0, 1e-12));
    assert!(close(est.stderr, (32.0_f64 / 56.0).sqrt(), 1e-12));
    assert_eq!(est.n, 8);
}

#[test]
fn mean_stderr_binary_scores() {
    // 3 of 4 correct: mean 0.75. var = (3*0.0625 + 1*0.5625)/3 = 0.25.
    let s = [1.0, 1.0, 1.0, 0.0];
    let est = mean_and_stderr(&s).unwrap();
    assert!(close(est.mean, 0.75, 1e-12));
    assert!(close(est.stderr, (0.25_f64 / 4.0).sqrt(), 1e-12));
}

#[test]
fn mean_stderr_needs_two() {
    assert!(mean_and_stderr(&[]).is_none());
    assert!(mean_and_stderr(&[1.0]).is_none());
}

// ---- clustered_stderr ------------------------------------------------------

#[test]
fn clustered_singletons_match_clt_exactly() {
    // With every item its own cluster, the G/(G-1) correction makes the
    // clustered SE reduce exactly to the CLT SE.
    let s = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
    let groups: Vec<usize> = (0..s.len()).collect();
    let clt = mean_and_stderr(&s).unwrap();
    let clustered = clustered_stderr(&s, &groups).unwrap();
    assert!(close(clustered.mean, clt.mean, 1e-12));
    assert!(close(clustered.stderr, clt.stderr, 1e-12));
}

#[test]
fn clustered_se_exceeds_clt_when_items_correlate() {
    // Two clusters, perfectly correlated within: clustering must widen the
    // bar relative to treating all 4 as independent.
    let s = [1.0, 1.0, 0.0, 0.0];
    let groups = [0usize, 0, 1, 1];
    let clt = mean_and_stderr(&s).unwrap();
    let clustered = clustered_stderr(&s, &groups).unwrap();
    assert!(clustered.stderr > clt.stderr);
}

#[test]
fn clustered_closed_form() {
    // s=[1,1,0,0], mean=0.5. Cluster residual sums: g0 = (0.5+0.5)=1.0,
    // g1 = (-0.5-0.5)=-1.0. Σ e_g² = 2. G=2, n=4.
    // var = (2/1) * 2 / 16 = 0.25 → SE = 0.5.
    let s = [1.0, 1.0, 0.0, 0.0];
    let groups = [0usize, 0, 1, 1];
    let est = clustered_stderr(&s, &groups).unwrap();
    assert!(close(est.stderr, 0.5, 1e-12));
}

#[test]
fn clustered_rejects_bad_input() {
    assert!(clustered_stderr(&[1.0, 2.0], &[0usize]).is_none()); // length mismatch
    assert!(clustered_stderr(&[1.0], &[0usize]).is_none()); // n < 2
    assert!(clustered_stderr(&[1.0, 2.0], &[0usize, 0]).is_none()); // G < 2
}

// ---- paired_difference -----------------------------------------------------

#[test]
fn paired_difference_reduces_to_mean_of_diffs() {
    let a = [0.9, 0.8, 0.7, 0.95];
    let b = [0.6, 0.7, 0.5, 0.65];
    let est = paired_difference(&a, &b).unwrap();
    // Diffs: 0.3, 0.1, 0.2, 0.3 → mean 0.225.
    assert!(close(est.mean, 0.225, 1e-12));
    let expected = mean_and_stderr(&[0.3, 0.1, 0.2, 0.3]).unwrap();
    assert!(close(est.stderr, expected.stderr, 1e-12));
}

#[test]
fn paired_difference_cuts_variance_vs_unpaired() {
    // Correlated conditions: pairing should yield a smaller SE than the SE
    // of either arm, because the shared per-item level cancels.
    let a = [10.0, 20.0, 30.0, 40.0];
    let b = [11.0, 21.0, 31.0, 41.0]; // b = a + 1 exactly
    let paired = paired_difference(&a, &b).unwrap();
    let arm = mean_and_stderr(&a).unwrap();
    assert!(close(paired.mean, -1.0, 1e-12));
    assert!(close(paired.stderr, 0.0, 1e-12)); // constant diff → zero SE
    assert!(paired.stderr < arm.stderr);
}

#[test]
fn paired_difference_rejects_mismatch() {
    assert!(paired_difference(&[1.0, 2.0], &[1.0]).is_none());
}

// ---- MDE / required_n ------------------------------------------------------

#[test]
fn mde_conventional_multiplier() {
    // alpha=0.05, power=0.80 → multiplier ≈ 2.8016.
    let mde = minimum_detectable_effect(1.0, 0.05, 0.80).unwrap();
    assert!(close(mde, 2.801_585, 1e-5));
}

#[test]
fn mde_scales_with_stderr() {
    let a = minimum_detectable_effect(0.5, 0.05, 0.80).unwrap();
    let b = minimum_detectable_effect(1.0, 0.05, 0.80).unwrap();
    assert!(close(b, 2.0 * a, 1e-9));
}

#[test]
fn required_n_closed_form() {
    // ((2.8015843 * 0.5 / 0.03)^2) = 2180.24 → ceil 2181.
    let n = required_n(0.5, 0.03, 0.05, 0.80).unwrap();
    assert_eq!(n, 2181);
}

#[test]
fn required_n_and_mde_are_consistent() {
    // If n items give SE = sd/sqrt(n), then MDE at that SE should land at
    // (just below, due to the ceil) the effect required_n was sized for.
    let sd = 0.5;
    let effect = 0.05;
    let n = required_n(sd, effect, 0.05, 0.80).unwrap();
    let se = sd / (n as f64).sqrt();
    let mde = minimum_detectable_effect(se, 0.05, 0.80).unwrap();
    // ceil(n) makes SE slightly smaller, so MDE is at or just under effect.
    assert!(mde <= effect + 1e-9);
    assert!(mde > effect * 0.97);
}

#[test]
fn required_n_rejects_bad_input() {
    assert!(required_n(0.0, 0.03, 0.05, 0.80).is_none());
    assert!(required_n(0.5, 0.0, 0.05, 0.80).is_none());
    assert!(required_n(0.5, f64::INFINITY, 0.05, 0.80).is_none());
}

// ---- confidence bound / clears_mde -----------------------------------------

#[test]
fn lower_bound_closed_form() {
    let est = Estimate {
        mean: 0.20,
        stderr: 0.05,
        n: 100,
    };
    // 0.20 - 1.959964 * 0.05 = 0.102002.
    let lb = lower_confidence_bound(&est, 0.05).unwrap();
    assert!(close(lb, 0.102_002, 1e-5));
}

#[test]
fn clears_mde_gate() {
    // Estimate well above the pre-registered MDE, tight SE: clears.
    let strong = Estimate {
        mean: 0.20,
        stderr: 0.02,
        n: 400,
    };
    assert!(clears_mde(&strong, 0.05, 0.05).unwrap());

    // Same point estimate, noisy: lower bound dips below the MDE → fails.
    let noisy = Estimate {
        mean: 0.20,
        stderr: 0.10,
        n: 30,
    };
    assert!(!clears_mde(&noisy, 0.05, 0.05).unwrap());
}

#[test]
fn clears_mde_rejects_a_just_significant_but_small_effect() {
    // Significantly positive (LB > 0) but the LB does not reach the
    // pre-registered MDE of 0.05 → the gate correctly refuses it.
    let est = Estimate {
        mean: 0.04,
        stderr: 0.01,
        n: 200,
    };
    let lb = lower_confidence_bound(&est, 0.05).unwrap();
    assert!(lb > 0.0); // it is significant
    assert!(!clears_mde(&est, 0.05, 0.05).unwrap()); // but does not clear MDE
}
