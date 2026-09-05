#!/usr/bin/env bash
# 建置 wasm 綁定：web（瀏覽器）與 nodejs（vitest 測試）兩種 target。
# 用法：scripts/build-wasm.sh [wasm-bindgen-cli 路徑]（預設用 PATH 中的 wasm-bindgen）
set -euo pipefail

WASM_BINDGEN="${1:-wasm-bindgen}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "==> cargo build (wasm32, release)"
cargo build -p gas2-wasm --target wasm32-unknown-unknown --release

WASM=target/wasm32-unknown-unknown/release/gas2_wasm.wasm

echo "==> wasm-bindgen → web/src/wasm (bundler/web target)"
rm -rf web/src/wasm
"$WASM_BINDGEN" --out-dir web/src/wasm --target web --out-name gas2_wasm "$WASM"

echo "==> wasm-bindgen → web/src/wasm-node (nodejs target, 給 vitest)"
rm -rf web/src/wasm-node
"$WASM_BINDGEN" --out-dir web/src/wasm-node --target nodejs --out-name gas2_wasm "$WASM"

echo "==> 完成"
ls -la web/src/wasm web/src/wasm-node
