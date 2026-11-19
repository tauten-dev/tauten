//! What a [`refined`](crate::refined) type may wrap.
//!
//! A refinement is checked once, in `try_new`. It stays true only if the
//! wrapped value cannot change afterwards and every copy of it is equal to
//! it. So the inner type must be [`Frozen`]: no interior mutability (`Cell`,
//! `RefCell`, atomics, `Mutex`, ...), which `get()` would expose through a
//! shared reference, and no user-written `Clone`, which a derived `Clone` of
//! the refined type would call. `Frozen` is sealed: it covers the primitive
//! types, `String`, and `Box`, `Option`, `Vec`, arrays, slices, tuples and
//! `&'static` references of `Frozen` types. Richer types need an explicit
//! preservation argument and are not supported yet.

mod sealed {
    pub trait Sealed {}
}

/// A type whose value cannot change behind a shared reference and whose
/// `Clone`, if any, copies the value. Sealed; see the [module docs](self).
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be wrapped by #[refined]: it is not `tauten::refine::Frozen`",
    note = "a refined value must not change after `try_new` checked it: interior mutability \
            (`Cell`, `RefCell`, atomics, locks) or a user-written `Clone` could break the predicate",
    note = "supported: primitives, `String`, and `Box`, `Option`, `Vec`, arrays, slices, tuples \
            and `&'static` references of those"
)]
pub trait Frozen: sealed::Sealed {}

/// Implemented by every `#[refined]` type; names the wrapped type.
pub trait Refined {
    /// The wrapped type.
    type Inner: Frozen + ?Sized;
}

macro_rules! frozen {
    ($($t:ty),* $(,)?) => {
        $(
            impl sealed::Sealed for $t {}
            impl Frozen for $t {}
        )*
    };
}

frozen!(
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    f32,
    f64,
    bool,
    char,
    str,
    String,
    (),
);

impl<T: Frozen + ?Sized> sealed::Sealed for Box<T> {}
impl<T: Frozen + ?Sized> Frozen for Box<T> {}
impl<T: Frozen> sealed::Sealed for Option<T> {}
impl<T: Frozen> Frozen for Option<T> {}
impl<T: Frozen> sealed::Sealed for Vec<T> {}
impl<T: Frozen> Frozen for Vec<T> {}
impl<T: Frozen, const N: usize> sealed::Sealed for [T; N] {}
impl<T: Frozen, const N: usize> Frozen for [T; N] {}
impl<T: Frozen> sealed::Sealed for [T] {}
impl<T: Frozen> Frozen for [T] {}
impl<T: Frozen + ?Sized> sealed::Sealed for &'static T {}
impl<T: Frozen + ?Sized> Frozen for &'static T {}

macro_rules! frozen_tuples {
    ($(($($t:ident),+)),* $(,)?) => {
        $(
            impl<$($t: Frozen),+> sealed::Sealed for ($($t,)+) {}
            impl<$($t: Frozen),+> Frozen for ($($t,)+) {}
        )*
    };
}

frozen_tuples!(
    (A),
    (A, B),
    (A, B, C),
    (A, B, C, D),
    (A, B, C, D, E),
    (A, B, C, D, E, F)
);
