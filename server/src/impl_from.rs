/// Implements 'From' for a list of variants, intended for use with error enums
/// that are wrapping a number of errors from other methods.
#[macro_export]
macro_rules! impl_from {
    (
        $enum_name: ident {
            $($error_type: ty => $variant_name: ident),* $(,)*
        }
    ) => {
        $(
            impl From<$error_type> for $enum_name {
                fn from(error: $error_type) -> $enum_name {
                    $enum_name::$variant_name(error)
                }
            }
        )*
    }
}