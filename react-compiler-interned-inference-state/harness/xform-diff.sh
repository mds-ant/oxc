#!/usr/bin/env bash
# Emitted-code differential over a file list using the two example drivers.
# Usage: xform-diff.sh <file-list> <out-prefix> [parallelism]
set -u
LIST=$1; PRE=$2; PAR=${3:-16}
H=/root/home/opensrc/oxc/.claude/worktrees/react-compiler-perf-8cdfbb/harness
W=/root/home/opensrc/oxc/.claude/worktrees/react-compiler-perf-8cdfbb/trees
xargs -a "$LIST" -d '\n' -P "$PAR" -n 1 "$H/xform-worker.sh" > "$PRE.tsv"
total=$(wc -l < "$PRE.tsv")
same=$(awk '$1==$2 && $3==$4' "$PRE.tsv" | wc -l)
diff=$(awk '$1!=$2 || $3!=$4' "$PRE.tsv" | wc -l)
base_nonzero=$(awk '$3!=0' "$PRE.tsv" | wc -l)
new_nonzero=$(awk '$4!=0' "$PRE.tsv" | wc -l)
echo "files=$total identical=$same different=$diff base_nonzero_exit=$base_nonzero patched_nonzero_exit=$new_nonzero"
awk '$1!=$2 || $3!=$4 {print $5}' "$PRE.tsv" > "$PRE.differing-files.txt"
i=0
while IFS= read -r f; do
  [ -z "$f" ] && continue
  i=$((i+1)); [ $i -gt 20 ] && break
  key=$(echo "$f" | md5sum | cut -c1-10)
  "$W/base/target/release/examples/react_compiler" "$f" > "$PRE.$key.base.out" 2>&1
  "$W/patched/target/release/examples/react_compiler" "$f" > "$PRE.$key.patched.out" 2>&1
  echo "differing: $f -> $PRE.$key.{base,patched}.out"
done < "$PRE.differing-files.txt"
