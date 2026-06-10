//! Validation-invariant guard (spec 002, US1, T004).
//!
//! A diff-scanning content guard: a self-heal fix must never weaken a
//! validation / range / contract check — the kind of check that *correctly*
//! rejects malformed output. The reproducing-test gate proves a behavior
//! *change*, not that the current behavior is wrong, so this is the only place to
//! catch a fix that "passes the test" by loosening a correct check (Constitution
//! III). Conservative by design: when it can see a check being weakened it flags
//! (and it over-flags rather than miss — research D1/D6). It runs on the fix's
//! proposed *production* file changes; a file with no current content (a new file
//! the fix adds) has no existing invariant to weaken and is cleared — new files
//! remain covered by the integrity path-guard and operator review. The reproducing
//! test is added separately and is not scanned here.

/// A file the fix proposes to change, with its full proposed contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedFile {
    /// Workspace-relative path.
    pub path: String,
    /// The full proposed new contents of the file.
    pub new_contents: String,
}

/// Verdict of the validation-invariant scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvariantVerdict {
    /// True if any changed file appears to weaken a validation/range/contract check.
    pub weakens: bool,
    /// Human-readable reason naming the file + pattern (FR-009); `None` when clear.
    pub reason: Option<String>,
}

impl InvariantVerdict {
    fn clear() -> Self {
        Self {
            weakens: false,
            reason: None,
        }
    }
    fn flag(reason: impl Into<String>) -> Self {
        Self {
            weakens: true,
            reason: Some(reason.into()),
        }
    }
}

/// Scan a fix's changed files for a weakened validation/range/contract check.
///
/// `read_current(path)` returns the current on-disk content, or `None` for a file
/// with no current content (a new file the fix adds). Pure over that closure so it
/// is unit-testable with no real filesystem. A `None` file is cleared (no existing
/// invariant to weaken); any *detected* weakening of an existing check returns
/// `weakens = true`.
pub fn scan_for_weakened_invariants(
    changed: &[ChangedFile],
    read_current: impl Fn(&str) -> Option<String>,
) -> InvariantVerdict {
    for cf in changed {
        let Some(current) = read_current(&cf.path) else {
            continue; // new file / no current content — no existing invariant to weaken
        };
        if let Some(reason) = file_weakens(&current, &cf.new_contents, &cf.path) {
            return InvariantVerdict::flag(reason);
        }
    }
    InvariantVerdict::clear()
}

/// `Some(reason)` when `proposed` weakens a validation check present in `current`.
fn file_weakens(current: &str, proposed: &str, path: &str) -> Option<String> {
    // (1) A validation/rejection line was removed outright.
    let cur_v = count_validation_lines(current);
    let new_v = count_validation_lines(proposed);
    if new_v < cur_v {
        return Some(format!(
            "{path}: a validation/rejection check was removed ({cur_v} → {new_v} guard lines)"
        ));
    }

    let removed = line_set_diff(current, proposed);
    let added = line_set_diff(proposed, current);

    // (2) A numeric range bound was widened in place.
    for r in &removed {
        for a in &added {
            if let Some(detail) = widened_range(r, a) {
                return Some(format!("{path}: {detail}"));
            }
        }
    }

    // (3) A guard comparison/negation was relaxed in place.
    for r in &removed {
        for a in &added {
            if relaxes_guard(r, a) {
                return Some(format!(
                    "{path}: a guard was relaxed ('{}' → '{}')",
                    r.trim(),
                    a.trim()
                ));
            }
        }
    }

    None
}

/// A line that participates in validation/rejection.
fn is_validation_line(line: &str) -> bool {
    let l = line.trim();
    l.contains("return Err")
        || l.contains("bail!")
        || l.contains("ensure!")
        || l.contains(".contains(")
        || l.contains("..=")
        || l.contains("InvalidValue")
        || l.contains("MissingField")
}

fn count_validation_lines(content: &str) -> usize {
    content.lines().filter(|l| is_validation_line(l)).count()
}

/// Raw lines of `a` whose trimmed form does not appear (trimmed) in `b`.
fn line_set_diff<'a>(a: &'a str, b: &str) -> Vec<&'a str> {
    let b_set: std::collections::HashSet<&str> = b.lines().map(str::trim).collect();
    a.lines().filter(|l| !b_set.contains(l.trim())).collect()
}

/// `Some(detail)` if a numeric range in `added` is a strict superset of one in
/// `removed` (a widened bound).
fn widened_range(removed: &str, added: &str) -> Option<String> {
    let old = extract_ranges(removed);
    let new = extract_ranges(added);
    for (lo, hi) in &old {
        for (lo2, hi2) in &new {
            if lo2 <= lo && hi2 >= hi && (lo2 < lo || hi2 > hi) {
                return Some(format!(
                    "a range bound was widened ({lo}..={hi} → {lo2}..={hi2})"
                ));
            }
        }
    }
    None
}

/// Parse `lo..=hi` / `lo..hi` numeric ranges from a line (numbers are ASCII).
fn extract_ranges(line: &str) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(pos) = line[i..].find("..") {
        let dot = i + pos;
        let lo = scan_number_left(line, dot);
        let mut right = dot + 2;
        if line.as_bytes().get(right) == Some(&b'=') {
            right += 1;
        }
        let hi = scan_number_right(line, right);
        if let (Some(lo), Some(hi)) = (lo, hi) {
            out.push((lo, hi));
        }
        i = dot + 2;
    }
    out
}

fn scan_number_left(line: &str, end_byte: usize) -> Option<f64> {
    let b = line.as_bytes();
    let mut j = end_byte;
    while j > 0 && (b[j - 1].is_ascii_digit() || b[j - 1] == b'.' || b[j - 1] == b'-') {
        j -= 1;
    }
    line[j..end_byte].trim().parse::<f64>().ok()
}

fn scan_number_right(line: &str, start_byte: usize) -> Option<f64> {
    let b = line.as_bytes();
    let mut j = start_byte;
    while j < b.len() && (b[j].is_ascii_digit() || b[j] == b'.' || b[j] == b'-') {
        j += 1;
    }
    line[start_byte..j].trim().parse::<f64>().ok()
}

/// True when `added` is `removed` with a relaxed comparison/negation — they are
/// identical once loosened operators are collapsed back to their strict form,
/// but differ in the raw text (`<`→`<=`, `>`→`>=`, `&&`→`||`, removed `!`).
fn relaxes_guard(removed: &str, added: &str) -> bool {
    let r = removed.trim();
    let a = added.trim();
    if r == a {
        return false;
    }
    let canon = |s: &str| {
        s.replace("<=", "<")
            .replace(">=", ">")
            .replace("||", "&&")
            .replace('!', "")
    };
    canon(r) == canon(a)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn changed(path: &str, contents: &str) -> Vec<ChangedFile> {
        vec![ChangedFile {
            path: path.to_string(),
            new_contents: contents.to_string(),
        }]
    }

    #[test]
    fn flags_widened_range_bound() {
        let current = "if !(0.0..=1.0).contains(&confidence) { return Err(e); }";
        let proposed = "if !(0.0..=100.0).contains(&confidence) { return Err(e); }";
        let v = scan_for_weakened_invariants(&changed("src/modes/linear.rs", proposed), |_| {
            Some(current.to_string())
        });
        assert!(v.weakens);
        let reason = v.reason.unwrap();
        assert!(reason.contains("src/modes/linear.rs"));
        assert!(reason.contains("range bound was widened"));
    }

    #[test]
    fn flags_removed_rejection_branch() {
        let current = "let x = parse();\nif x.is_invalid() { return Err(e); }\nOk(x)";
        let proposed = "let x = parse();\nOk(x)"; // the return Err guard deleted
        let v = scan_for_weakened_invariants(&changed("src/m.rs", proposed), |_| {
            Some(current.to_string())
        });
        assert!(v.weakens);
        assert!(v.reason.unwrap().contains("removed"));
    }

    #[test]
    fn flags_relaxed_comparison() {
        let current = "if confidence < 0.0 { return Err(e); }";
        let proposed = "if confidence <= 0.0 { return Err(e); }";
        let v = scan_for_weakened_invariants(&changed("src/m.rs", proposed), |_| {
            Some(current.to_string())
        });
        assert!(v.weakens);
        assert!(v.reason.unwrap().contains("relaxed"));
    }

    #[test]
    fn flags_dropped_negation() {
        let current = "if !(0.0..=1.0).contains(&c) { reject(); }";
        let proposed = "if (0.0..=1.0).contains(&c) { reject(); }"; // ! dropped → inverted guard
        let v = scan_for_weakened_invariants(&changed("src/m.rs", proposed), |_| {
            Some(current.to_string())
        });
        assert!(v.weakens);
    }

    #[test]
    fn clears_new_file_with_no_current_content() {
        // No current content (a new file the fix adds) → no existing invariant to
        // weaken. New files remain covered by the integrity path-guard + review.
        let v = scan_for_weakened_invariants(&changed("src/new_helper.rs", "fn x() {}"), |_| None);
        assert!(!v.weakens);
        assert!(v.reason.is_none());
    }

    #[test]
    fn clears_fix_that_touches_no_validation_line() {
        let current = "let x = compute(a, b);\nOk(x)";
        let proposed = "let x = compute(a, b) + adjust();\nOk(x)";
        let v = scan_for_weakened_invariants(&changed("src/m.rs", proposed), |_| {
            Some(current.to_string())
        });
        assert!(
            !v.weakens,
            "a non-validation edit must not be flagged (SC-005)"
        );
        assert!(v.reason.is_none());
    }

    #[test]
    fn clears_identical_content() {
        let same = "if !(0.0..=1.0).contains(&c) { return Err(e); }";
        let v =
            scan_for_weakened_invariants(&changed("src/m.rs", same), |_| Some(same.to_string()));
        assert!(!v.weakens);
    }

    #[test]
    fn extract_ranges_parses_float_bounds() {
        assert_eq!(extract_ranges("(0.0..=1.0)"), vec![(0.0, 1.0)]);
        assert_eq!(extract_ranges("0..=100"), vec![(0.0, 100.0)]);
        assert!(extract_ranges("no range here").is_empty());
    }
}
