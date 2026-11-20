//! Declared layers inside a tier (TAU-T004).
//!
//! A workspace may list named layers in order
//! (`[workspace.metadata.tauten] layers = ["model", "store", "service"]`).
//! A crate names its layer; it may depend on a crate of the same or an
//! earlier layer. Tiers remain the trust boundary; layers order crates
//! within the rules the tiers already set.
//!
//! The shell turns each declaration into a [`Layer`]: the position in the
//! list, a name that is not in it, or no declaration. Proved: the order
//! over positions is reflexive, transitive, antisymmetric and total
//! (`may_use_*`), and a dependency is a finding exactly when both crates
//! have a known layer and the dependency's is later, or either declaration
//! is unknown or missing (`layer_finding_iff`).

/// A crate's declared layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    /// The position of its layer in the workspace's list.
    Known(usize),
    /// A `layer` that is not in the list.
    Unknown,
    /// No `layer` although the workspace declares layers.
    Missing,
}

/// Whether a crate of layer position `of` may use one at position `dep`.
#[must_use]
pub const fn may_use(of: usize, dep: usize) -> bool {
    dep <= of
}

/// TAU-T004 for one dependency edge: a finding unless both layers are
/// known and the dependency's is not later.
#[must_use]
pub const fn finding(of: Layer, dep: Layer) -> bool {
    match (of, dep) {
        (Layer::Known(a), Layer::Known(b)) => !may_use(a, b),
        (Layer::Unknown | Layer::Missing, _) | (_, Layer::Unknown | Layer::Missing) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layers_point_backwards() {
        assert!(!finding(Layer::Known(2), Layer::Known(0)));
        assert!(!finding(Layer::Known(1), Layer::Known(1)));
        assert!(finding(Layer::Known(0), Layer::Known(1)));
        assert!(finding(Layer::Unknown, Layer::Known(0)));
        assert!(finding(Layer::Known(3), Layer::Missing));
    }
}
