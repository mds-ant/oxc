#!/usr/bin/env bash
# Differential runner: lint the corpus with one binary + config, keep the raw
# JSON, the exit code, stderr, and the canonical order-independent set.
# Usage: run.sh <oxlint-binary> <label> <threads> [config.json]
set -u
BIN=$1
LABEL=$2
THREADS=$3
CFG=${4:-/root/home/opensrc/oxc/.claude/worktrees/react-compiler-perf-8cdfbb/harness/rc-only.json}
H=/root/home/opensrc/oxc/.claude/worktrees/react-compiler-perf-8cdfbb/harness
OUT=$H/out
mkdir -p "$OUT"
cd "$H"

CAL=/root/home/opensrc/oxc/.claude/worktrees/react-compiler-perf/target/corpus/cal.com
EXC=/root/home/opensrc/oxc/.claude/worktrees/react-compiler-perf/target/corpus/excalidraw
OUTLINE=/root/home/opensrc/oxc/.claude/worktrees/react-compiler-perf-8cdfbb/corpus/outline
GRAFANA=/root/home/opensrc/oxc/.claude/worktrees/react-compiler-perf-8cdfbb/corpus/grafana

start=$(date +%s.%N)
"$BIN" --disable-nested-config -c "$CFG" --format json --threads "$THREADS" \
  "$CAL" "$EXC" "$OUTLINE" "$GRAFANA" > "$OUT/$LABEL.json" 2> "$OUT/$LABEL.err"
code=$?
end=$(date +%s.%N)
echo "$code" > "$OUT/$LABEL.exit"
wall=$(python3 -c "print(f'{$end - $start:.2f}')")
summary=$(python3 "$H/canon.py" "$OUT/$LABEL.json" "$OUT/$LABEL.exit" "$OUT/$LABEL.canon")
echo "$LABEL: wall=${wall}s $summary (stderr bytes=$(wc -c < "$OUT/$LABEL.err"))"
