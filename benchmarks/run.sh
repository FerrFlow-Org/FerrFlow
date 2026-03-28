#!/usr/bin/env bash
set -euo pipefail

# FerrFlow Benchmark Runner
# Compares ferrflow against semantic-release, changesets, and release-please.
#
# Usage: ./run.sh [--json] [--fixtures-dir <path>]
#
# Outputs a Markdown table to stdout (or JSON with --json).
# Requires: ferrflow, node, npx, hyperfine, /usr/bin/time (GNU)

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
FIXTURES_DIR="$SCRIPT_DIR/fixtures"
RESULTS_DIR="$SCRIPT_DIR/results"
OUTPUT_FORMAT="markdown"
WARMUP_RUNS=2
BENCHMARK_RUNS=5

while [[ $# -gt 0 ]]; do
  case "$1" in
    --json) OUTPUT_FORMAT="json"; shift ;;
    --fixtures-dir) FIXTURES_DIR="$2"; shift 2 ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
done

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

command_exists() { command -v "$1" &>/dev/null; }

require_cmd() {
  if ! command_exists "$1"; then
    echo "Required command not found: $1" >&2
    exit 1
  fi
}

# Measure execution time in milliseconds
measure_time() {
  local start end
  start=$(date +%s%N)
  "$@" &>/dev/null || true
  end=$(date +%s%N)
  awk "BEGIN {printf \"%.1f\", ($end - $start) / 1000000}"
}

# Measure peak RSS in MB (Linux only)
measure_memory() {
  if [[ "$(uname)" == "Linux" ]]; then
    /usr/bin/time -v "$@" 2>&1 >/dev/null | grep "Maximum resident" | awk '{print $6}' | awk '{printf "%.1f", $1/1024}'
  else
    echo "N/A"
  fi
}

# Get binary/install size in MB
get_size() {
  local cmd="$1"
  local path
  path=$(command -v "$cmd" 2>/dev/null || echo "")
  if [[ -n "$path" && -f "$path" ]]; then
    du -m "$path" | awk '{printf "%.1f", $1}'
  else
    echo "N/A"
  fi
}

get_node_pkg_size() {
  local pkg="$1"
  local tmp_dir
  tmp_dir=$(mktemp -d)
  cd "$tmp_dir"
  npm init -y &>/dev/null
  npm install --save "$pkg" &>/dev/null 2>&1
  local size
  size=$(du -sm node_modules | awk '{printf "%.1f", $1}')
  cd - >/dev/null
  rm -rf "$tmp_dir"
  echo "$size"
}

# ---------------------------------------------------------------------------
# Generate fixtures if missing
# ---------------------------------------------------------------------------

if [[ ! -d "$FIXTURES_DIR/single" ]]; then
  echo "Generating fixtures..." >&2
  bash "$FIXTURES_DIR/generate.sh" "$FIXTURES_DIR"
fi

# ---------------------------------------------------------------------------
# Benchmark a single tool on a fixture
# ---------------------------------------------------------------------------

# Returns: cold_ms warm_ms memory_mb
bench_ferrflow() {
  local fixture="$1"
  cd "$fixture"

  # Cold start (drop caches if possible)
  local cold
  cold=$(measure_time ferrflow check 2>/dev/null)

  # Warm runs
  local warm_total=0
  for _ in $(seq 1 $BENCHMARK_RUNS); do
    local t
    t=$(measure_time ferrflow check 2>/dev/null)
    warm_total=$(awk "BEGIN {print $warm_total + $t}")
  done
  local warm
  warm=$(awk "BEGIN {printf \"%.1f\", $warm_total / $BENCHMARK_RUNS}")

  # Memory
  local mem
  mem=$(measure_memory ferrflow check)

  cd - >/dev/null
  echo "$cold $warm $mem"
}

bench_node_tool() {
  local fixture="$1" tool_cmd="$2"

  # Create a temporary working copy to avoid polluting the fixture
  local tmp_dir
  tmp_dir=$(mktemp -d)
  cp -a "$fixture/." "$tmp_dir/"
  cd "$tmp_dir"

  # Cold start
  local cold
  cold=$(measure_time $tool_cmd 2>/dev/null)

  # Warm runs
  local warm_total=0
  for _ in $(seq 1 $BENCHMARK_RUNS); do
    local t
    t=$(measure_time $tool_cmd 2>/dev/null)
    warm_total=$(awk "BEGIN {print $warm_total + $t}")
  done
  local warm
  warm=$(awk "BEGIN {printf \"%.1f\", $warm_total / $BENCHMARK_RUNS}")

  # Memory
  local mem
  mem=$(measure_memory $tool_cmd)

  cd - >/dev/null
  rm -rf "$tmp_dir"
  echo "$cold $warm $mem"
}

# ---------------------------------------------------------------------------
# Run benchmarks
# ---------------------------------------------------------------------------

require_cmd ferrflow
require_cmd awk

FERRFLOW_SIZE=$(get_size ferrflow)

# Check which competitors are available
declare -A TOOLS
TOOLS["ferrflow"]="ferrflow check"

if command_exists npx; then
  TOOLS["semantic-release"]="npx --yes semantic-release --dry-run --no-ci"
  TOOLS["changesets"]="npx --yes @changesets/cli status"
  TOOLS["release-please"]="npx --yes release-please release-pr --dry-run --repo-url=."
fi

FIXTURES=("single" "mono-small" "mono-large")
FIXTURE_LABELS=("single" "mono-small (10 pkg)" "mono-large (50 pkg)")

declare -A RESULTS

echo "Running benchmarks..." >&2

for i in "${!FIXTURES[@]}"; do
  fixture_name="${FIXTURES[$i]}"
  fixture_path="$FIXTURES_DIR/$fixture_name"

  if [[ ! -d "$fixture_path" ]]; then
    echo "Fixture not found: $fixture_path, skipping" >&2
    continue
  fi

  echo "  Fixture: $fixture_name" >&2

  # ferrflow
  echo "    ferrflow..." >&2
  read -r cold warm mem <<< "$(bench_ferrflow "$fixture_path")"
  RESULTS["ferrflow|$fixture_name"]="$cold|$warm|$FERRFLOW_SIZE|$mem"

  # Node-based competitors (may not be available in CI)
  if command_exists npx; then
    for tool in "semantic-release" "changesets" "release-please"; do
      echo "    $tool..." >&2
      case "$tool" in
        semantic-release)
          read -r cold warm mem <<< "$(bench_node_tool "$fixture_path" "npx --yes semantic-release --dry-run --no-ci")" || true
          ;;
        changesets)
          read -r cold warm mem <<< "$(bench_node_tool "$fixture_path" "npx --yes @changesets/cli status")" || true
          ;;
        release-please)
          read -r cold warm mem <<< "$(bench_node_tool "$fixture_path" "npx --yes release-please release-pr --dry-run")" || true
          ;;
      esac
      RESULTS["$tool|$fixture_name"]="${cold:-N/A}|${warm:-N/A}|N/A|${mem:-N/A}"
    done
  fi
done

# ---------------------------------------------------------------------------
# Output
# ---------------------------------------------------------------------------

if [[ "$OUTPUT_FORMAT" == "json" ]]; then
  echo "{"
  first=true
  for key in "${!RESULTS[@]}"; do
    IFS='|' read -r tool fixture <<< "$key"
    IFS='|' read -r cold warm size mem <<< "${RESULTS[$key]}"
    if ! $first; then echo ","; fi
    first=false
    printf '  "%s": {"fixture": "%s", "cold_ms": "%s", "warm_ms": "%s", "size_mb": "%s", "memory_mb": "%s"}' \
      "$key" "$fixture" "$cold" "$warm" "$size" "$mem"
  done
  echo ""
  echo "}"
else
  for i in "${!FIXTURES[@]}"; do
    fixture_name="${FIXTURES[$i]}"
    fixture_label="${FIXTURE_LABELS[$i]}"

    echo ""
    echo "### ${fixture_label}"
    echo ""
    echo "| Tool | Cold start | Warm start | Binary/Install | Memory (RSS) |"
    echo "|------|-----------|------------|----------------|-------------|"

    for tool in "ferrflow" "semantic-release" "changesets" "release-please"; do
      key="$tool|$fixture_name"
      if [[ -v "RESULTS[$key]" ]]; then
        IFS='|' read -r cold warm size mem <<< "${RESULTS[$key]}"
        echo "| $tool | ${cold}ms | ${warm}ms | ${size} MB | ${mem} MB |"
      fi
    done
  done
fi

# ---------------------------------------------------------------------------
# Save baseline
# ---------------------------------------------------------------------------

mkdir -p "$RESULTS_DIR"
TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)
FERRFLOW_VERSION=$(ferrflow --version 2>/dev/null | head -1 || echo "unknown")

cat > "$RESULTS_DIR/latest.json" <<JSON
{
  "timestamp": "$TIMESTAMP",
  "ferrflow_version": "$FERRFLOW_VERSION",
  "fixtures": {
$(for i in "${!FIXTURES[@]}"; do
  fixture_name="${FIXTURES[$i]}"
  key="ferrflow|$fixture_name"
  if [[ -v "RESULTS[$key]" ]]; then
    IFS='|' read -r cold warm size mem <<< "${RESULTS[$key]}"
    echo "    \"$fixture_name\": {\"cold_ms\": $cold, \"warm_ms\": $warm, \"size_mb\": \"$size\", \"memory_mb\": \"$mem\"}$([ $i -lt $((${#FIXTURES[@]}-1)) ] && echo ",")"
  fi
done)
  }
}
JSON

echo "" >&2
echo "Results saved to $RESULTS_DIR/latest.json" >&2
