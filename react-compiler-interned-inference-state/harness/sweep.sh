#!/usr/bin/env bash
# Synthetic size sweep: single-file, --threads 1, best-of-R wall time per binary.
# Usage: sweep.sh <out.tsv> <reps> <max-seconds-per-run> <file>...
set -u
OUT=$1; REPS=$2; MAXS=$3; shift 3
H=/root/home/opensrc/oxc/.claude/worktrees/react-compiler-perf-8cdfbb/harness
BASE=/root/home/opensrc/oxc/.claude/worktrees/react-compiler-perf-8cdfbb/bin/oxlint-baseline
PATCHED=/root/home/opensrc/oxc/.claude/worktrees/react-compiler-perf-8cdfbb/bin/oxlint-patched
printf 'file\tlines\tbase_ms\tpatched_ms\tratio\n' > "$OUT"
timeit() {
  local bin=$1 file=$2 best=999999999 i s e ms
  for ((i=0; i<REPS; i++)); do
    s=$(date +%s%N)
    timeout "$MAXS" "$bin" --disable-nested-config -c "$H/rc-only.json" --format json --threads 1 "$file" > /dev/null 2>&1
    rc=$?
    e=$(date +%s%N)
    if [ $rc -eq 124 ]; then echo "TIMEOUT"; return; fi
    ms=$(( (e - s) / 1000000 ))
    (( ms < best )) && best=$ms
  done
  echo "$best"
}
for f in "$@"; do
  lines=$(wc -l < "$f")
  p=$(timeit "$PATCHED" "$f")
  b=$(timeit "$BASE" "$f")
  if [ "$b" = "TIMEOUT" ] || [ "$p" = "TIMEOUT" ]; then ratio="-"; else ratio=$(python3 -c "print(f'{$b/max($p,1):.1f}')"); fi
  printf '%s\t%s\t%s\t%s\t%s\n' "$(basename "$f")" "$lines" "$b" "$p" "$ratio" | tee -a "$OUT"
done
