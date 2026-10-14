#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
SUITE="${1:?Supply engine, cli, or adapter}"
case "$SUITE" in
  engine) ARGS=(-p webfont-generator --lib --tests);;
  # Keep binary tests and feature-gated CLI integration tests, not engine tests.
  cli) ARGS=(-p webfont-generator --features cli --bins --test integration -E 'kind(bin) | (binary(=integration) & test(/^cli::/))');;
  adapter) ARGS=(-p webfont-generator-napi --lib);;
  *) echo "Unknown Rust coverage suite: $SUITE" >&2; exit 1;;
esac
OUT="$ROOT/coverage/rust-$SUITE"
mkdir -p "$OUT"
rm -f "$OUT"/junit*.xml "$OUT/rust.lcov"
RUN="$(mktemp -d "$OUT/run.XXXXXX")"
trap 'rm -rf "$RUN"' EXIT
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
MODES=(default)
if [[ "$SUITE" == engine ]]; then
  MODES+=(fs in_memory in_memory,async cli)
fi
for mode in "${MODES[@]}"; do
  RUN_ARGS=("${ARGS[@]}")
  REPORT="$OUT/junit.xml"
  if [[ "$mode" != default ]]; then
    RUN_ARGS+=(--no-default-features --features "$mode")
    REPORT="$OUT/junit-${mode//,/-}.xml"
  fi
  # --no-report retains earlier coverage profiles; never reuse their test results.
  rm -f "$RUN/nextest/ci/junit.xml"
  cargo llvm-cov nextest --locked --no-report --profile ci --config-file "$RUN/nextest.toml" "${RUN_ARGS[@]}" || status=$?
  # nextest writes no JUnit if compilation prevents execution. Do not synthesize passing tests.
  if [[ -s "$RUN/nextest/ci/junit.xml" ]]; then
    cp "$RUN/nextest/ci/junit.xml" "$REPORT"
  else
    echo "Rust tests did not produce JUnit: $SUITE ($mode)" >&2
    status=1
  fi
done
cargo llvm-cov report -p webfont-generator -p webfont-generator-napi --lcov --output-path "$RUN/rust.lcov"
node scripts/ci/validate-native-coverage.mjs "$RUN/rust.lcov" "$SUITE"
cp "$RUN/rust.lcov" "$OUT/rust.lcov"
exit "$status"
