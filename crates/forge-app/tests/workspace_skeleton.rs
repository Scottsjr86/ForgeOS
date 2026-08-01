use forge_bridge as _;
use forge_core as _;
use forge_editor as _;
use forge_git as _;
use forge_nyx_client as _;
use forge_project as _;
use forge_protocol as _;
use forge_session as _;
use forge_terminal as _;
use forge_world as _;

#[test]
fn composition_root_links_runtime_authority_crates() {
    assert_eq!(env!("CARGO_PKG_NAME"), "forge-app");
}
