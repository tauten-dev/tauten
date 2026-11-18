//! Refinement types. Each predicate is the one its theorems name.

/// A percentage: at most 100.
#[tauten::refined(|v| *v <= 100)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Percent(u8);

/// A registered (non-system, non-ephemeral) port: 1024 to 49151.
#[tauten::refined(|p| *p >= 1024 && *p <= 49151)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Port(u16);

/// A signed offset of at most 1000 in either direction.
#[tauten::refined(|x| *x >= -1000 && *x <= 1000)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Offset(i32);

/// A count that is never zero.
#[tauten::refined(|n| *n != 0)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Count(u64);

/// Control for the constructor audit: a plain newtype may derive
/// `Default`, and the audit must find `new` and the derived `default` as
/// its constructors, or the enumeration is vacuous.
#[tauten::newtype]
#[derive(Clone, Copy, Default)]
pub struct Tag(u32);

#[cfg(test)]
mod tests {
    use super::{Count, Offset, Percent, Port, Tag};

    #[test]
    fn percent_boundaries() {
        assert_eq!(Percent::try_new(0).map(|p| *p.get()), Ok(0));
        assert_eq!(Percent::try_new(100).map(|p| *p.get()), Ok(100));
        assert!(Percent::try_new(101).is_err());
        assert!(Percent::try_new(255).is_err());
        assert!(Percent::try_from(100).is_ok());
    }

    #[test]
    fn port_boundaries() {
        assert!(Port::try_new(1023).is_err());
        assert!(matches!(
            Port::try_new(1024).map(Port::into_inner),
            Ok(1024)
        ));
        assert!(matches!(
            Port::try_new(49151).map(Port::into_inner),
            Ok(49151)
        ));
        assert!(Port::try_new(49152).is_err());
    }

    #[test]
    fn offset_boundaries() {
        assert!(Offset::try_new(-1001).is_err());
        assert!(matches!(
            Offset::try_new(-1000).map(Offset::into_inner),
            Ok(-1000)
        ));
        assert!(matches!(
            Offset::try_new(1000).map(Offset::into_inner),
            Ok(1000)
        ));
        assert!(Offset::try_new(1001).is_err());
    }

    #[test]
    fn count_boundaries() {
        assert!(Count::try_new(0).is_err());
        assert!(matches!(Count::try_new(1).map(Count::into_inner), Ok(1)));
        assert_eq!(
            Count::try_new(u64::MAX).map(Count::into_inner),
            Ok(u64::MAX)
        );
    }

    #[test]
    fn tag_default_is_zero() {
        assert_eq!(Tag::default().into_inner(), 0);
    }
}
