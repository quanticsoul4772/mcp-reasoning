//! Statistical primitives for the eval harness.
//!
//! These are the foundation the rest of the harness (and the self-improvement
//! reward rewrite) consume: a quality estimate is never reported without a
//! standard error, and a change is never accepted unless a *paired* measured
//! delta clears a pre-registered Minimum Detectable Effect (MDE).
//!
//! The recipe follows Evan Miller, "Adding Error Bars to Evals,"
//! [arXiv:2411.00640](https://arxiv.org/abs/2411.00640):
//!
//! - report the standard error of the mean via the central limit theorem;
//! - use **clustered** standard errors when items are correlated within groups
//!   (e.g. several questions drawn from one passage);
//! - compare two conditions with a **paired** per-item difference, which removes
//!   the between-item variance and is the lever that makes small effects
//!   detectable at feasible sample sizes;
//! - size experiments with a power analysis (MDE / required-n) so a run that
//!   *cannot* detect the effect you care about says so, instead of reporting a
//!   reassuring-but-meaningless point estimate.
//!
//! Every function is pure and total over its preconditions: when there is not
//! enough data to form an estimate (empty input, `n < 2`, mismatched lengths,
//! out-of-range probabilities) the result is `None` rather than a panic or a
//! silently-wrong number. Callers map `None` to a contextual error.

/// A point estimate paired with its standard error and the sample size it was
/// computed from.
///
/// `stderr` is the estimated standard deviation of `mean` as an estimator — the
/// quantity that becomes the error bar. `n` is retained for reporting and for
/// downstream power calculations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Estimate {
    /// The point estimate (a mean score, or a paired mean difference).
    pub mean: f64,
    /// Standard error of `mean`.
    pub stderr: f64,
    /// Number of observations the estimate was computed from.
    pub n: usize,
}

// Acklam's rational-approximation coefficients, reproduced verbatim from the
// published algorithm; the trailing digits exceed f64 precision and round
// harmlessly, so the excessive-precision lint is allowed here only.
#[allow(clippy::excessive_precision)]
const ACKLAM_A: [f64; 6] = [
    -3.969_683_028_665_376e1,
    2.209_460_984_245_205e2,
    -2.759_285_104_469_687e2,
    1.383_577_518_672_690e2,
    -3.066_479_806_614_716e1,
    2.506_628_277_459_239e0,
];
#[allow(clippy::excessive_precision)]
const ACKLAM_B: [f64; 5] = [
    -5.447_609_879_822_406e1,
    1.615_858_368_580_409e2,
    -1.556_989_798_598_866e2,
    6.680_131_188_771_972e1,
    -1.328_068_155_288_572e1,
];
#[allow(clippy::excessive_precision)]
const ACKLAM_C: [f64; 6] = [
    -7.784_894_002_430_293e-3,
    -3.223_964_580_411_365e-1,
    -2.400_758_277_161_838e0,
    -2.549_732_539_343_734e0,
    4.374_664_141_464_968e0,
    2.938_163_982_698_783e0,
];
#[allow(clippy::excessive_precision)]
const ACKLAM_D: [f64; 4] = [
    7.784_695_709_041_462e-3,
    3.224_671_290_700_398e-1,
    2.445_134_137_142_996e0,
    3.754_408_661_907_416e0,
];
const ACKLAM_P_LOW: f64 = 0.024_25;
const ACKLAM_P_HIGH: f64 = 1.0 - ACKLAM_P_LOW;

/// Inverse of the standard normal CDF (the probit / quantile function).
///
/// Returns `z` such that `Φ(z) = p`. Uses Acklam's rational approximation,
/// accurate to a relative error of about `1.15e-9` across `0 < p < 1`.
///
/// Returns `None` for `p <= 0.0`, `p >= 1.0`, or a non-finite `p` (the quantile
/// is `±∞` / undefined there).
#[must_use]
// a/b/c/d/p/q/r mirror the published Acklam notation; renaming them would only
// obscure the correspondence to the reference algorithm.
#[allow(clippy::many_single_char_names)]
pub fn inverse_normal_cdf(p: f64) -> Option<f64> {
    if !p.is_finite() || p <= 0.0 || p >= 1.0 {
        return None;
    }

    let (a, b, c, d) = (&ACKLAM_A, &ACKLAM_B, &ACKLAM_C, &ACKLAM_D);

    let z = if p < ACKLAM_P_LOW {
        // Lower tail.
        let q = (-2.0 * p.ln()).sqrt();
        (((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
            / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1.0)
    } else if p <= ACKLAM_P_HIGH {
        // Central region.
        let q = p - 0.5;
        let r = q * q;
        (((((a[0] * r + a[1]) * r + a[2]) * r + a[3]) * r + a[4]) * r + a[5]) * q
            / (((((b[0] * r + b[1]) * r + b[2]) * r + b[3]) * r + b[4]) * r + 1.0)
    } else {
        // Upper tail.
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
            / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1.0)
    };
    Some(z)
}

/// Mean and standard error of a sample, via the central limit theorem.
///
/// `stderr = s / sqrt(n)`, where `s` is the sample standard deviation
/// (Bessel-corrected, `n - 1` denominator). Works identically over binary
/// `{0.0, 1.0}` scores and continuous scores.
///
/// Returns `None` when `samples.len() < 2` (a standard error is undefined for a
/// single observation).
#[must_use]
pub fn mean_and_stderr(samples: &[f64]) -> Option<Estimate> {
    let n = samples.len();
    if n < 2 {
        return None;
    }
    let n_f = n as f64;
    let mean = samples.iter().sum::<f64>() / n_f;
    let sum_sq = samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>();
    let variance = sum_sq / (n_f - 1.0);
    Some(Estimate {
        mean,
        stderr: (variance / n_f).sqrt(),
        n,
    })
}

/// Mean and **clustered** standard error, for samples whose items are correlated
/// within groups.
///
/// Uses the cluster-robust ("sandwich") estimator for the variance of the mean
/// with the standard `G / (G - 1)` small-sample correction:
///
/// ```text
/// stderr = sqrt(  G/(G-1) * (1/n^2) * Σ_g ( Σ_{i in g} (x_i - mean) )^2  )
/// ```
///
/// where `G` is the number of distinct clusters. Two cases worth knowing:
///
/// - When every item is its own cluster (`G == n`), this reduces **exactly** to
///   [`mean_and_stderr`] — clustering only changes the answer when items share a
///   group.
/// - When correlated items share a cluster, the clustered SE is (correctly)
///   *larger* than the naive CLT SE; ignoring the clustering would understate the
///   error bar.
///
/// `group_ids` labels each sample's cluster and must be the same length as
/// `samples`. Returns `None` on a length mismatch, fewer than 2 samples, or
/// fewer than 2 distinct clusters (the correction is undefined for `G < 2`).
#[must_use]
pub fn clustered_stderr<T: Eq + std::hash::Hash>(
    samples: &[f64],
    group_ids: &[T],
) -> Option<Estimate> {
    let n = samples.len();
    if n < 2 || group_ids.len() != n {
        return None;
    }
    let n_f = n as f64;
    let mean = samples.iter().sum::<f64>() / n_f;

    // Sum of residuals per cluster.
    let mut residual_sums: std::collections::HashMap<&T, f64> = std::collections::HashMap::new();
    for (x, g) in samples.iter().zip(group_ids.iter()) {
        *residual_sums.entry(g).or_insert(0.0) += x - mean;
    }
    let g_count = residual_sums.len();
    if g_count < 2 {
        return None;
    }
    let g_f = g_count as f64;

    let cluster_sum_sq = residual_sums.values().map(|e| e * e).sum::<f64>();
    let variance = (g_f / (g_f - 1.0)) * cluster_sum_sq / (n_f * n_f);
    Some(Estimate {
        mean,
        stderr: variance.sqrt(),
        n,
    })
}

/// Paired per-item mean difference (`a` minus `b`) and its standard error.
///
/// This is the variance-reduction lever from Miller §"Paired Differences": by
/// differencing the two conditions on the **same** items, the between-item
/// variance cancels, so the SE of the difference is typically far smaller than
/// the SE of either condition alone — which is what makes small effects
/// detectable without enormous datasets.
///
/// `a` and `b` must be item-aligned (same length, `a[i]` and `b[i]` are the two
/// conditions on item `i`). Returns `None` on a length mismatch or fewer than 2
/// items.
#[must_use]
pub fn paired_difference(a: &[f64], b: &[f64]) -> Option<Estimate> {
    if a.len() != b.len() {
        return None;
    }
    let diffs: Vec<f64> = a.iter().zip(b.iter()).map(|(x, y)| x - y).collect();
    mean_and_stderr(&diffs)
}

/// Two-sided z critical value plus the one-sided power z, the multiplier shared
/// by the MDE and required-n formulas. `None` if `alpha`/`power` are out of
/// `(0, 1)`.
fn power_z(alpha: f64, power: f64) -> Option<f64> {
    let z_alpha = inverse_normal_cdf(1.0 - alpha / 2.0)?;
    let z_power = inverse_normal_cdf(power)?;
    Some(z_alpha + z_power)
}

/// Minimum Detectable Effect for an estimate with the given standard error.
///
/// The smallest true effect that a two-sided test at significance `alpha` would
/// reject the null for with probability `power`:
///
/// ```text
/// MDE = (z_{1 - alpha/2} + z_{power}) * stderr
/// ```
///
/// For the conventional `alpha = 0.05`, `power = 0.80` the multiplier is
/// `≈ 2.8016`. This is the post-run feasibility number: compute it *after* a run
/// and, if it exceeds the effect you pre-registered as meaningful, the correct
/// conclusion is "this dataset cannot test this," not a point estimate.
///
/// Returns `None` for `stderr < 0` or out-of-range `alpha`/`power`.
#[must_use]
pub fn minimum_detectable_effect(stderr: f64, alpha: f64, power: f64) -> Option<f64> {
    if stderr < 0.0 {
        return None;
    }
    Some(power_z(alpha, power)? * stderr)
}

/// Number of items required to detect `effect` with the given `power` at
/// significance `alpha`, for a per-item standard deviation `std_dev`.
///
/// ```text
/// n = ceil( ( (z_{1 - alpha/2} + z_{power}) * std_dev / effect )^2 )
/// ```
///
/// For a paired comparison, pass the standard deviation of the **per-item
/// differences** as `std_dev`; its smallness (relative to the raw score SD) is
/// exactly why pairing cuts the required `n`.
///
/// Returns `None` for `std_dev <= 0`, `effect == 0`, a non-finite `effect`, or
/// out-of-range `alpha`/`power`.
#[must_use]
pub fn required_n(std_dev: f64, effect: f64, alpha: f64, power: f64) -> Option<usize> {
    if std_dev <= 0.0 || !effect.is_finite() || effect == 0.0 {
        return None;
    }
    let z = power_z(alpha, power)?;
    let raw = (z * std_dev / effect).powi(2).ceil();
    if !raw.is_finite() || raw < 1.0 {
        return None;
    }
    // `raw` is the square of a real number rounded up: finite and >= 1 here, so
    // the cast neither loses sign nor truncates a fraction.
    #[allow(clippy::cast_sign_loss)]
    Some(raw as usize)
}

/// Lower bound of the two-sided `1 - alpha` confidence interval for the estimate.
///
/// `mean - z_{1 - alpha/2} * stderr`. Returns `None` for out-of-range `alpha`.
#[must_use]
pub fn lower_confidence_bound(estimate: &Estimate, alpha: f64) -> Option<f64> {
    let z = inverse_normal_cdf(1.0 - alpha / 2.0)?;
    Some(estimate.mean - z * estimate.stderr)
}

/// Whether a measured estimate **clears** a pre-registered MDE: the accept gate.
///
/// Returns `true` iff the lower bound of the `1 - alpha` confidence interval is
/// at or above `pre_registered_mde`. This is deliberately stricter than "the
/// effect is significantly positive": the change must be confidently *at least
/// as large* as the effect you committed to caring about before the run, which
/// is what stops the self-improvement loop from rewarding effects too small to
/// matter or too noisy to trust.
///
/// `pre_registered_mde` is fixed before the run (see [`minimum_detectable_effect`]
/// for the post-run feasibility number that informs the choice). Returns `None`
/// for out-of-range `alpha`.
#[must_use]
pub fn clears_mde(estimate: &Estimate, pre_registered_mde: f64, alpha: f64) -> Option<bool> {
    Some(lower_confidence_bound(estimate, alpha)? >= pre_registered_mde)
}
