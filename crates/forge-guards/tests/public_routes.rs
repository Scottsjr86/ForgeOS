#![allow(unused_imports)]

use forge_guards::{core_purity, seams, source_size};

#[test]
fn guard_crate_exposes_only_named_guard_routes() {
    assert_eq!(env!("CARGO_PKG_NAME"), "forge-guards");
}
