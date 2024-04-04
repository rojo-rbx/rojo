use crate::rojo_test::syncback_util::basic_syncback_test;

macro_rules! syncback_basic_test {
    ($($test_name:ident$(,)?)*) => {$(
        #[test]
        fn $test_name() {
            let _ = env_logger::try_init();

            basic_syncback_test(stringify!($test_name)).unwrap()
        }
    )*};
}

syncback_basic_test! {
    baseplate,
    respect_old_middleware,
    nested_projects,
    unscriptable_properties,
    nested_projects_weird,
    project_init,
    all_middleware,
    ref_properties,
    rbxm_fallback
}
