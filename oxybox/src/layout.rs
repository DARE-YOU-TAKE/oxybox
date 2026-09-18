//! Compile-time checks that a Rust mirror struct matches the `b2*` struct it is handed to Box2D as.

/// Asserts that `$rust_ty` has the same size, alignment, and field offsets as `$c_ty`.
///
/// Normal fields map as `rust_name => c_name`. An opaque placeholder standing in for a run of C
/// fields names its type and the C field that follows the run, as in
/// `mixing_callbacks: MixingCallbacks => frictionCallback .. enableSleep`.
macro_rules! mirrors_layout {
    ($rust_ty:ty => $c_ty:ty { $($fields:tt)* }) => {
        const _: () = {
            assert!(::std::mem::size_of::<$rust_ty>() == ::std::mem::size_of::<$c_ty>());
            assert!(::std::mem::align_of::<$rust_ty>() == ::std::mem::align_of::<$c_ty>());
            $crate::layout::mirrors_layout!(@fields $rust_ty, $c_ty, $($fields)*);
        };
    };

    (@fields $rust_ty:ty, $c_ty:ty,) => {};

    // an opaque placeholder standing in for the C fields in `$c_field .. $c_end`
    (@fields $rust_ty:ty, $c_ty:ty, $field:ident : $blob:ty => $c_field:ident .. $c_end:ident, $($rest:tt)*) => {
        assert!(::std::mem::offset_of!($rust_ty, $field) == ::std::mem::offset_of!($c_ty, $c_field));
        assert!(
            ::std::mem::size_of::<$blob>()
                == ::std::mem::offset_of!($c_ty, $c_end) - ::std::mem::offset_of!($c_ty, $c_field)
        );
        $crate::layout::mirrors_layout!(@fields $rust_ty, $c_ty, $($rest)*);
    };

    (@fields $rust_ty:ty, $c_ty:ty, $field:ident => $c_field:ident, $($rest:tt)*) => {
        assert!(::std::mem::offset_of!($rust_ty, $field) == ::std::mem::offset_of!($c_ty, $c_field));
        $crate::layout::mirrors_layout!(@fields $rust_ty, $c_ty, $($rest)*);
    };
}

pub(crate) use mirrors_layout;
