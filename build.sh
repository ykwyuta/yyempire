#!/usr/bin/env bash
# WebAssembly 版をビルドして dist/ に静的サイトを出力する。
# Cloudflare Pages ではビルドコマンドに `bash build.sh`、出力ディレクトリに `dist` を指定する。
set -euo pipefail
cd "$(dirname "$0")"

if ! command -v cargo >/dev/null 2>&1; then
  if [ -f "$HOME/.cargo/env" ]; then
    . "$HOME/.cargo/env"
  else
    echo "Rust が無いので rustup で導入する"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
    . "$HOME/.cargo/env"
  fi
fi
rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true

cargo build --release --target wasm32-unknown-unknown

rm -rf dist
mkdir -p dist
cp web/index.html web/main.js dist/
cp assets/app_icon.ico dist/favicon.ico
cp target/wasm32-unknown-unknown/release/yyempire.wasm dist/
# wasm-opt があればさらに小さくする（任意）
if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -O3 dist/yyempire.wasm -o dist/yyempire.wasm || true
fi
cat > dist/_headers <<'H'
/yyempire.wasm
  Content-Type: application/wasm
  Cache-Control: public, max-age=3600
H
ls -la dist
