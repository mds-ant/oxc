#!/usr/bin/env bash
# Per-file single-thread timing of both binaries over a file list.
# Usage: perfile.sh <file-list> <out.tsv> [repeats]
# Output columns: base_ms  patched_ms  ratio  bytes  path   (min over repeats)
set -u
LIST=$1
OUT=$2
REPS=${3:-1}
H=/root/home/opensrc/oxc/.claude/worktrees/react-compiler-perf-8cdfbb/harness
BASE=/root/home/opensrc/oxc/.claude/worktrees/react-compiler-perf-8cdfbb/bin/oxlint-baseline
PATCHED=/root/home/opensrc/oxc/.claude/worktrees/react-compiler-perf-8cdfbb/bin/oxlint-patched
: > "$OUT"
timeit() {
  local bin=$1 file=$2 best=999999 i s e ms
  for ((i=0; i<REPS; i++)); do
    s=$(date +%s%N)
    "$bin" --disable-nested-config -c "$H/rc-only.json" --format json --threads 1 "$file" > /dev/null 2>&1
    e=$(date +%s%N)
    ms=$(( (e - s) / 1000000 ))
    (( ms < best )) && best=$ms
  done
  echo "$best"
}
while IFS= read -r f; do
  [ -z "$f" ] && continue
  b=$(timeit "$BASE" "$f")
  p=$(timeit "$PATCHED" "$f")
  bytes=$(wc -c < "$f")
  ratio=$(python3 -c "print(f'{$b/max($p,1):.2f}')")
  printf '%s\t%s\t%s\t%s\t%s\n' "$b" "$p" "$ratio" "$bytes" "$f" >> "$OUT"
done < "$LIST"
sort -t$'\t' -k1,1 -rn "$OUT" -o "$OUT"
