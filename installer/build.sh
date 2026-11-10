#!/bin/bash
# Builds the Windows installer from the release build: installer/SkipDat-<version>-setup.exe
# Needs NSIS 3 (makensis). The app itself is built first with cargo (see README).
set -e
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$HERE/.."
VERSION=$(grep -m1 '^version' "$ROOT/src-tauri/Cargo.toml" | cut -d'"' -f2)
REL="${REL:-$ROOT/src-tauri/target/x86_64-pc-windows-gnu/release}"
[ -f "$REL/skip-dat.exe" ] || REL="$ROOT/src-tauri/target/release"
STAGE="$HERE/stage"
rm -rf "$STAGE" && mkdir -p "$STAGE"
cp "$REL/skip-dat.exe" "$STAGE/Skip Dat.exe"
cp "$REL/WebView2Loader.dll" "$STAGE/"
cp "$HERE/readme.txt" "$STAGE/readme.txt"
cp -r "$ROOT/browser-companion" "$STAGE/browser-companion"
ANIM="$HERE/anim/target/x86_64-pc-windows-gnu/release/setup-anim.exe"
[ -f "$ANIM" ] || ANIM="$HERE/anim/target/release/setup-anim.exe"
cp "$ANIM" "$STAGE/setup-anim.exe"
cd "$HERE"
makensis -V2 -DVERSION="$VERSION" -DSRC=stage -DOUTFILE="${OUTFILE:-SkipDat-$VERSION-setup.exe}" ${EXTRA:-} skipdat.nsi
rm -rf "$STAGE"
ls -la "$HERE/${OUTFILE:-SkipDat-$VERSION-setup.exe}"
