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
    all_middleware,
    unscriptable_properties,
    respect_old_middleware,
    rbxm_fallback
    project_init,
    nested_projects,
    nested_projects_weird,
    ref_properties,
    ref_properties_blank,
    ref_properties_update,
    ignore_paths,
    project_reserialize,
    project_all_middleware,
    duplicate_rojo_id,
    string_value_project,
}
