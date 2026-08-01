use forge_protocol as _;

#[test]
fn protocol_boundary_is_linkable() {
    assert_eq!(env!("CARGO_PKG_NAME"), "forge-protocol");
}
