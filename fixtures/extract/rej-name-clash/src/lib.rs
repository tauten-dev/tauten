//! A parameter named like its module: in the Lean the translation produces,
//! the parameter `span` shadows the namespace `span`, so `span.Span` no
//! longer names the type.
#![no_std]

/// Spans.
pub mod span {
    /// A half-open range.
    #[derive(Clone, Copy)]
    pub struct Span {
        /// Start.
        pub start: u32,
        /// End.
        pub end: u32,
    }

    /// The width of `span`, or 0 for an empty or inverted one.
    #[must_use]
    pub const fn width(span: Span) -> u32 {
        span.end.saturating_sub(span.start)
    }

    /// Whether `span` is wider than `other`.
    #[must_use]
    pub const fn wider(span: Span, other: Span) -> bool {
        width(span) > width(other)
    }
}
