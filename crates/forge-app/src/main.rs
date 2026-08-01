//! ForgeOS executable entrypoint.
//!
//! Product behavior belongs to owning crates, not this binary root.

fn main() {
    forge_app::composition::run();
}
