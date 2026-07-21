#!/usr/bin/env bash
# Compile one file with the base and patched example drivers; print
#   <md5-of-base-output> <md5-of-patched-output> <base-exit> <patched-exit> <file>
# Output = stdout+stderr of the driver. Differences are re-run and saved by
# the driver script.
f=$1
W=/root/home/opensrc/oxc/.claude/worktrees/react-compiler-perf-8cdfbb/trees
BASE=$W/base/target/release/examples/react_compiler
NEW=$W/patched/target/release/examples/react_compiler
ob=$("$BASE" "$f" 2>&1); rb=$?
on=$("$NEW" "$f" 2>&1); rn=$?
hb=$(printf '%s' "$ob" | md5sum | cut -d' ' -f1)
hn=$(printf '%s' "$on" | md5sum | cut -d' ' -f1)
printf '%s %s %s %s %s\n' "$hb" "$hn" "$rb" "$rn" "$f"
