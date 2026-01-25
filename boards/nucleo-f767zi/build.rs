//! Makes the board-specific linker contract visible to `rust-lld`.

use std::path::Path;

fn main() {
    let linker_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("linker");

    println!("cargo:rustc-link-search={}", linker_dir.display());
    println!("cargo:rerun-if-changed=linker/link.x");
    println!("cargo:rerun-if-changed=linker/memory.x");
}
