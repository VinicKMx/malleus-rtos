//! Makes the board-specific linker contract visible to `rust-lld`.

use std::path::Path;

fn main() {
    let probes = [
        "CARGO_FEATURE_HARDFAULT_ESCALATION_PROBE",
        "CARGO_FEATURE_BUSFAULT_READ_PROBE",
        "CARGO_FEATURE_PREARM_FAULT_PROBE",
        "CARGO_FEATURE_REENTRANT_FAULT_PROBE",
        "CARGO_FEATURE_STARTUP_DATA_FAILURE_PROBE",
        "CARGO_FEATURE_INTERRUPTED_CAPTURE_PROBE",
    ];
    let selected = probes
        .iter()
        .filter(|name| std::env::var_os(name).is_some())
        .count();
    assert!(selected <= 1, "select at most one diagnostic probe feature");

    let linker_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("linker");

    println!("cargo:rustc-link-search={}", linker_dir.display());
    println!("cargo:rerun-if-changed=linker/link.x");
    println!("cargo:rerun-if-changed=linker/memory.x");
}
