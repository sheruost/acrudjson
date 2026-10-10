//! Compile-time reflection, from the nightly `type_info` feature.

use core::any::TypeId;

/// The names of `T`'s fields, read at compile time.
///
/// `N` must equal the number of fields, or the build fails.
pub(crate) const fn field_names<T: ?Sized + 'static, const N: usize>() -> [&'static str; N] {
    const {
        let id = TypeId::of::<T>();
        assert!(id.fields(0) == N, "N must equal the number of fields");
        let mut names = [""; N];
        let mut i = 0;
        while i < N {
            names[i] = id.field(0, i).name();
            i += 1;
        }
        names
    }
}
