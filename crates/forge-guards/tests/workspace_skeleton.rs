use forge_guards as _;

#[test]
fn structural_guard_boundary_is_linkable() {
    assert_eq!(env!("CARGO_PKG_NAME"), "forge-guards");
}
