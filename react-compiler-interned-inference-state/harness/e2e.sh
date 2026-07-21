#!/usr/bin/env bash
# End-to-end interleaved timing: N rounds, alternating which binary runs first,
# each timed run gated on the host 1-minute load average.
# Usage: e2e.sh <threads> <rounds> <out.tsv>
set -u
THREADS=$1; ROUNDS=$2; OUT=$3
H=/root/home/opensrc/oxc/.claude/worktrees/react-compiler-perf-8cdfbb/harness
B=/root/home/opensrc/oxc/.claude/worktrees/react-compiler-perf-8cdfbb/bin
CAL=/root/home/opensrc/oxc/.claude/worktrees/react-compiler-perf/target/corpus/cal.com
EXC=/root/home/opensrc/oxc/.claude/worktrees/react-compiler-perf/target/corpus/excalidraw
OUTLINE=/root/home/opensrc/oxc/.claude/worktrees/react-compiler-perf-8cdfbb/corpus/outline
GRAFANA=/root/home/opensrc/oxc/.claude/worktrees/react-compiler-perf-8cdfbb/corpus/grafana
CFG=$H/rc-only.json

run() { # <label>
  local bin=$B/oxlint-$1 s e
  while [ "$(cut -d' ' -f1 /proc/loadavg | cut -d. -f1)" -ge 16 ]; do sleep 5; done
  s=$(date +%s%N)
  "$bin" --disable-nested-config -c "$CFG" --format json --threads "$THREADS" \
    "$CAL" "$EXC" "$OUTLINE" "$GRAFANA" > /dev/null 2>&1
  e=$(date +%s%N)
  echo $(( (e - s) / 1000000 ))
}

printf 'round\tfirst\tbaseline_ms\tpatched_ms\tload\n' > "$OUT"
run baseline > /dev/null; run patched > /dev/null   # warm-up
for ((r=1; r<=ROUNDS; r++)); do
  if (( r % 2 == 1 )); then first=baseline; b=$(run baseline); p=$(run patched)
  else first=patched; p=$(run patched); b=$(run baseline); fi
  printf '%s\t%s\t%s\t%s\t%s\n' "$r" "$first" "$b" "$p" "$(cut -d' ' -f1 /proc/loadavg)" | tee -a "$OUT"
done
python3 - "$OUT" <<'EOF'
import sys, statistics
rows = [l.split('\t') for l in open(sys.argv[1]).read().splitlines()[1:]]
b = [int(r[2]) for r in rows]; p = [int(r[3]) for r in rows]
wins = sum(1 for x, y in zip(b, p) if y < x)
print(f"baseline: median {statistics.median(b)} ms (min {min(b)}, max {max(b)})")
print(f"patched:  median {statistics.median(p)} ms (min {min(p)}, max {max(p)})")
print(f"patched faster in {wins}/{len(rows)} paired rounds; median ratio {statistics.median(b)/statistics.median(p):.2f}x")
EOF
