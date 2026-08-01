use forge_core as _;
use forge_protocol as _;

#[test]
fn pure_core_boundary_is_linkable() {
    assert_eq!(env!("CARGO_PKG_NAME"), "forge-core");
}
