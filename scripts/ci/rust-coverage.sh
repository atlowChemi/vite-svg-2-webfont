#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
SUITE="${1:?Supply engine, cli, or adapter}"
case "$SUITE" in
  engine) ARGS=(-p webfont-generator --lib --tests);;
  cli) ARGS=(-p webfont-generator --features cli --lib --bins --tests);;
  adapter) ARGS=(-p webfont-generator-napi --lib);;
  *) echo "Unknown Rust coverage suite: $SUITE" >&2; exit 1;;
esac
OUT="$ROOT/coverage/rust-$SUITE"
mkdir -p "$OUT"
rm -f "$OUT/junit.xml" "$OUT/rust.lcov"
RUN="$(mktemp -d "$OUT/run.XXXXXX")"
export CARGO_TARGET_DIR="$RUN/target"
export CARGO_LLVM_COV_TARGET_DIR="$CARGO_TARGET_DIR"
# nextest's store is independent of Cargo's target directory. Give this execution
# its own store as well, so simultaneous suites cannot overwrite each other's JUnit.
node --input-type=module - "$RUN" <<'JS'
import { readFileSync, writeFileSync } from 'node:fs';
const run = process.argv[2];
writeFileSync(`${run}/nextest.toml`, `[store]\ndir = ${JSON.stringify(`${run}/nextest`)}\n${readFileSync('.config/nextest.toml', 'utf8')}`);
JS
status=0
cargo llvm-cov nextest --locked --no-report --profile ci --config-file "$RUN/nextest.toml" "${ARGS[@]}" || status=$?
# nextest writes no JUnit if compilation prevents execution. Do not synthesize passing tests.
if [[ -s "$RUN/nextest/ci/junit.xml" ]]; then
  cp "$RUN/nextest/ci/junit.xml" "$OUT/junit.xml"
  cargo llvm-cov report -p webfont-generator -p webfont-generator-napi --lcov --output-path "$RUN/rust.lcov"
  node scripts/ci/validate-native-coverage.mjs "$RUN/rust.lcov" "$SUITE"
  cp "$RUN/rust.lcov" "$OUT/rust.lcov"
else
  echo "Rust tests did not produce JUnit: $SUITE" >&2
  exit 1
fi
exit "$status"
