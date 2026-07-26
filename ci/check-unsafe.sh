#!/usr/bin/env bash
# Verify that `unsafe` appears only in the crates permitted to contain it.
#
# ADR-0010 confines unsafe code to `malleusrt` and the `malleus-arch-*` crates.
# Host tooling and the manifest/analysis crates must have none: they run on a
# developer's machine, where there is nothing an unsafe block could buy that is
# worth the review burden.
#
# Detection matches actual unsafe *constructs* — `unsafe {`, `unsafe fn`,
# `unsafe impl`, `unsafe trait` — and ignores comments. A naive `grep unsafe`
# also fires on documentation about the unsafe policy, which is exactly the
# prose a project like this contains a lot of.

set -euo pipefail

# Crates that must not contain unsafe code.
readonly CLEAN_PATHS=(
    crates/malleus-manifest
    crates/malleus-analyzer
    crates/malleus-codegen
    crates/malleus-trace
    tools
    xtask
)

# `unsafe` followed by the start of a block, function, impl, or trait, not
# preceded by a word character (so `unsafe_audit` does not match) and not on a
# line whose first non-whitespace is a comment marker.
readonly PATTERN='^[[:space:]]*([^/*].*)?(^|[^[:alnum:]_])unsafe[[:space:]]*(\{|fn[[:space:]]|impl[[:space:]]|trait[[:space:]])'

status=0
for path in "${CLEAN_PATHS[@]}"; do
    [ -d "$path" ] || continue
    if matches=$(grep -rnE --include='*.rs' "$PATTERN" "$path" 2>/dev/null); then
        echo "error: unsafe code found in a crate that must not contain it:"
        echo "$matches"
        status=1
    fi
done

if [ "$status" -ne 0 ]; then
    echo
    echo "See docs/adr/0010-unsafe-code-policy.md. If this crate genuinely needs"
    echo "unsafe, that is a design change and needs an ADR, not an exception here."
    exit 1
fi

echo "OK: no unsafe outside malleusrt and malleus-arch-*."

# Report the inventory, so the number is visible in every CI run rather than
# only when somebody goes looking. A growing count is not automatically wrong —
# an unremarked growing count is.
echo
echo "Unsafe inventory (crates permitted to contain it):"
for path in crates/malleusrt crates/malleus-arch crates/malleus-arch-cortex-m; do
    [ -d "$path" ] || continue
    # `|| true`: grep exits non-zero when a crate has no unsafe at all, which
    # is a fine state to be in, not an error. pipefail would otherwise abort.
    count=$( { grep -rcE --include='*.rs' "$PATTERN" "$path" 2>/dev/null || true; } \
        | awk -F: '{s+=$2} END {print s+0}')
    printf '  %-32s %s construct(s)\n' "$path" "$count"
done
