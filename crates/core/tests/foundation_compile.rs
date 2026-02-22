use dobo_core::{dsl, resolver, validation};

#[test]
fn modules_are_importable() {
    assert_eq!(dsl::module_name(), "dsl");
    assert_eq!(resolver::module_name(), "resolver");
    assert_eq!(validation::module_name(), "validation");
}
