#![allow(unused_imports)]

use forge_app::composition;
use forge_bridge::{adapters, ports};
use forge_core::{capabilities, missions, projects, workspaces};
use forge_editor::{buffers, language};
use forge_git::{repository, worktree};
use forge_nyx_client::{protocol as nyx_protocol, transport as nyx_transport};
use forge_project::{persistence, registry, repository_view, text_search};
use forge_protocol::{envelopes, errors, events, identities, intents};
use forge_session::{lifecycle, services};
use forge_terminal::{commands, pty};
use forge_world::{interaction, presentation};

#[test]
fn production_crates_expose_named_public_routes() {
    composition::run();
    assert_eq!(env!("CARGO_PKG_NAME"), "forge-app");
}
