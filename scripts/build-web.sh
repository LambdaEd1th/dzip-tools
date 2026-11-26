#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd -P)"
app="$root/crates/dzip-gui"
output="$root/target/nsh-web"
bundle="$output/Dzip Archive"
nsh="${NSH_CLI:-nsh}"
if [[ "$("$nsh" --version)" != "nsh 0.7.1 "* ]]; then
    echo "Dzip Archive requires Native Shell CLI 0.7.1." >&2
    exit 1
fi

# The CLI's generated launcher is a separate Cargo workspace. Seed its
# dependency versions from the same lock as the archive worker.
mkdir -p "$app/.nsh/rust"
cp "$root/Cargo.lock" "$app/.nsh/rust/Cargo.lock"
if [[ "$(uname -s)" == Darwin ]]; then
    # Some Command Line Tools installations ship clang without the WebAssembly
    # backend or llvm-ar. Prefer the LLVM toolchain when installed via Homebrew.
    if command -v brew >/dev/null && llvm_prefix="$(brew --prefix llvm 2>/dev/null)" && [[ -x "$llvm_prefix/bin/llvm-ar" ]]; then
        export CC_wasm32_unknown_unknown="${CC_wasm32_unknown_unknown:-$llvm_prefix/bin/clang}"
        export AR_wasm32_unknown_unknown="${AR_wasm32_unknown_unknown:-$llvm_prefix/bin/llvm-ar}"
    fi
fi
export CC_wasm32_unknown_unknown="${CC_wasm32_unknown_unknown:-clang}"
export AR_wasm32_unknown_unknown="${AR_wasm32_unknown_unknown:-llvm-ar}"
(cd "$app" && "$nsh" package --target web --out "$output")

cd "$root"
cargo build --locked --release -p dzip-gui-worker --target wasm32-unknown-unknown
worker="$bundle/assets/worker"
mkdir -p "$worker"
cp "$app/web/dzip-worker.js" "$worker/"
wasm-bindgen --target web --no-typescript --out-dir "$worker/pkg" \
    "${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/release/dzip_gui_worker.wasm"
touch "$bundle/.nojekyll"
printf 'Web app: %s\n' "$bundle"
