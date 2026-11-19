//! Which function a finding is in: the innermost span covering a line.
//!
//! A proof subsumes findings only inside the function it is about, so
//! choosing the wrong function could silence a finding. The shell lists
//! each function's first and last line; [`innermost`] picks the narrowest
//! span that covers the line, the first such span on a tie. It is proved
//! to cover the line, to be no wider than any other covering span, and to
//! be `None` exactly when no span covers the line.

/// Whether the span `(first, last)` covers `line`.
#[must_use]
pub const fn covers(lines: (u32, u32), line: u32) -> bool {
    lines.0 <= line && line <= lines.1
}

/// The number of lines after the first.
#[must_use]
pub const fn width(lines: (u32, u32)) -> u32 {
    lines.1.saturating_sub(lines.0)
}

/// `best`, or `i` if span `i` covers `line` and is strictly narrower.
#[must_use]
pub fn consider(spans: &[(u32, u32)], line: u32, best: Option<usize>, i: usize) -> Option<usize> {
    let Some(candidate) = spans.get(i) else {
        return best;
    };
    if !covers(*candidate, line) {
        return best;
    }
    let Some(b) = best else {
        return Some(i);
    };
    match spans.get(b) {
        Some(current) if width(*current) <= width(*candidate) => best,
        Some(_) | None => Some(i),
    }
}

/// The index of the narrowest span covering `line` (the first on a tie),
/// or `None` when none covers it.
#[must_use]
pub fn innermost(spans: &[(u32, u32)], line: u32) -> Option<usize> {
    let mut best: Option<usize> = None;
    let mut i: usize = 0;
    while i < spans.len() {
        best = consider(spans, line, best, i);
        i = i.saturating_add(1);
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_narrowest() {
        let spans = [(1, 3), (8, 13), (10, 12), (17, 19)];
        assert_eq!(innermost(&spans, 11), Some(2));
        assert_eq!(innermost(&spans, 9), Some(1));
        assert_eq!(innermost(&spans, 5), None);
        assert_eq!(innermost(&[(1, 5), (2, 6)], 3), Some(0));
        assert_eq!(innermost(&[], 3), None);
    }
}
