#!/usr/bin/env bash
# Standalone clippy of the vendored crate (pristine and patched copies), since
# a vendored registry dep is compiled with --cap-lints allow inside the
# workspace build. Copies live under ~/.cache/rc-clippy/{base,patched} with a
# .cargo/config.toml pointing crates-io at the corresponding tree's vendor dir.
set -u
R=$HOME/.cache/rc-clippy
S=/tmp/claude-999/-root-home-opensrc-oxc--claude-worktrees-react-compiler-perf-8cdfbb/8339db17-5669-4d54-a8e7-8db84182b4a1/scratchpad
for s in base patched; do
  cd $R/$s/forked_react_compiler_inference
  # give the copy a workspace root so cargo doesn't search upward
  grep -q '^\[workspace\]' Cargo.toml || printf '\n[workspace]\n' >> Cargo.toml
  CARGO_TARGET_DIR=$R/target-$s cargo +1.96.0 clippy --offline --lib > $S/clippy-$s.log 2>&1
  echo "$s: exit=$? warnings=$(grep -c '^warning' $S/clippy-$s.log) errors=$(grep -c '^error' $S/clippy-$s.log)"
done
echo "== warnings unique to the patched copy =="
grep -A4 '^warning' $S/clippy-patched.log | grep -E '^warning|-->' | paste - - | sort > $S/clippy-patched.warnings
grep -A4 '^warning' $S/clippy-base.log | grep -E '^warning|-->' | paste - - | sort > $S/clippy-base.warnings
comm -13 <(sed 's/ *--> .*//' $S/clippy-base.warnings | sort -u) <(sed 's/ *--> .*//' $S/clippy-patched.warnings | sort -u)
