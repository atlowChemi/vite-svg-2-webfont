#!/usr/bin/env bash
# Linux/macOS native coverage. Never place NAPI_RS_NATIVE_LIBRARY_PATH in the environment:
# Vite+ uses NAPI too. The coverage config redirects only this package's loader.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="$ROOT/coverage/napi-vitest"
mkdir -p "$OUT"
rm -f "$OUT/rust.lcov" "$OUT/junit.xml"
rm -rf "$OUT/js"
RUN="$(mktemp -d "$OUT/run.XXXXXX")"
export CARGO_TARGET_DIR="$RUN/target"
export CARGO_LLVM_COV_TARGET_DIR="$CARGO_TARGET_DIR"
cd "$ROOT/packages/webfont-generator"
eval "$(cargo llvm-cov show-env --sh --no-rustc-wrapper)"
TARGET="$(rustc -vV | sed -n 's/^host: //p')"
vp exec napi build --platform --esm --js binding.js --dts binding.d.ts --output-dir "$RUN/addon" -- --locked
export WEBFONT_NATIVE_COVERAGE_DIR="$RUN"
cd "$ROOT"
status=0
vp test --run --config=packages/webfont-generator/vite.coverage.config.ts || status=$?
# Export partial results even when an executed test fails; retain its exit status.
test -s "$RUN/junit.xml"
cp "$RUN/junit.xml" "$OUT/junit.xml"
test -s "$RUN/js/lcov.info"
node scripts/ci/validate-native-coverage.mjs "$RUN/js/lcov.info" js
cp -R "$RUN/js" "$OUT/js"
cargo llvm-cov report -p webfont-generator -p webfont-generator-napi --target "$TARGET" --lcov --output-path "$RUN/rust.lcov"
node scripts/ci/validate-native-coverage.mjs "$RUN/rust.lcov"
cp "$RUN/rust.lcov" "$OUT/rust.lcov"
exit "$status"
