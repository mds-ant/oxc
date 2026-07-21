# Interned inference state for `InferMutationAliasingEffects` — verification

Independent reproduction of a performance patch to `forked_react_compiler_inference` 0.2.0
(`src/infer_mutation_aliasing_effects.rs`), the Rust port of the React Compiler consumed by
oxlint v1.71.0's `react/react-compiler` rule. The patch replaces the pass's deep-copied
inference state (per-value `AbstractValue { kind, reasons }` and per-variable value sets) with
hash-consed values/sets in a per-pass `ValuePool`, referenced by `Copy` handles.

- `interned-inference-state.patch` — the change under test (apply `-p0` at the repo root of oxc
  `c4be770f24` with `forked_react_compiler_inference` 0.2.0 vendored under `vendor/`).
- `harness/` — the scripts used below. `measure-timing.patch` is a small oxc-side instrument
  (env var enables the compiler's built-in per-pass timers in the example driver), applied
  identically to both trees.
- `results/` — raw result tables.

Machine: 64-vCPU x86_64 (shared host with other jobs running; timed runs are
interleaved and load-gated). Ratios transfer across machines; absolute times do not.

---

## 0. Premise finding — where "upstream" actually is

The crate this patch targets is stale in every direction:

- **`oxc-project/forked-react-compiler`** is a codemod-driven **mirror** of Meta's own Rust
  port in `facebook/react` (`compiler/crates/react_compiler_inference/…`) plus four small
  patches (`patches/`). Its copy of this file is byte-identical to 0.2.0 (git blob `ca1bd2ca…`);
  no fix ever landed there, and only docs/chore commits followed 0.2.0 (2026-06-18).
- **oxc no longer consumes the crate.** `#23993` (after v1.71.0) vendored the compiler into
  `crates/oxc_react_compiler/src/react_compiler_*`, and oxc `main` already carries the same fix
  idea: `#24117` *perf(react_compiler): avoid copying the aliasing fixpoint's abstract state*
  (merged 2026-07-12/13, `Rc`-shared value sets), plus `#24480` (reasons as a `u16` bitmask
  `ReasonSet`) and `#24506` (effect interning).
- **The origin — `facebook/react` `main` — still carries the pathology** (deep-copied
  `FxHashMap<ValueId, AbstractValue>` / `FxHashMap<IdentifierId, FxHashSet<ValueId>>`,
  `state = incomingState.clone()` per visit, clone-on-null-merge in `queue`). Only two commits
  ever touched the file there.

Verdicts per destination are in §6.

## 1. Apply / build / artifact check

- oxc `c4be770f24` (tag `oxlint_v1.71.0`) in two detached worktrees; toolchain 1.96.0 (the
  tag's pin); `cargo vendor` with crates-io redirected to `vendored-sources`. Identity checks
  before patching all matched exactly: crate `version = "0.2.0"`, package checksum
  `b17f5d24…`, file sha256 `8d2f9b46…` (3,665 lines).
- Patch applied cleanly with both `git apply -p0` and GNU `patch` (65 hunks, no fuzz, no
  offsets); resulting diff **+492 / −234** (file now 3,923 lines). `.cargo-checksum.json`
  entry refreshed (`8d2f9b46…` → `4bf79c8a…`).
- Both binaries built with `cargo build --profile release-with-debug -p oxlint --bin oxlint
  --features allocator`, each in its own fresh `target/`; both `Finished`, exit 0. Artifact
  check: `grep -a -c ValuePool` → **0** on the baseline binary, **non-zero** on the patched one.

## 2. Correctness

Diagnostics are compared as order-independent canonical sets (file, rule code, severity,
message, help, sorted label spans), plus exit code and summary counts (`canon.py`, `run.sh`).
Corpus A = cal.com + excalidraw + outline + grafana `public/app` (16,566 lintable files,
6,500 tsx/jsx); corpus B = posthog `frontend/src` + appsmith `app/client/src` (9,660 files).

| gate | result |
|---|---|
| `cargo test -p oxc_linter react_compiler` (rule fixtures + `react_react_compiler.snap`), patched tree | pass — 1 test, snapshot unchanged |
| baseline vs baseline, corpus A, 16 threads | set-identical (1,971 diagnostics) — corpus + harness deterministic |
| **baseline vs patched, corpus A, 16 threads** | **set-identical** (1,971 diagnostics, exit 1 both) |
| baseline-1thr vs patched-1thr — determinism probe (one thread makes `ValueId` allocation deterministic and directly exercises the changed set-iteration order) | set-identical |
| baseline-1thr vs baseline-16thr; patched-1thr vs patched-16thr | set-identical |
| `reportAllBailouts: true`, corpus A (adds compiler bailout diagnostics) | set-identical (2,549 diagnostics) |
| corpus B, both configs | set-identical (1,457 / 1,643 diagnostics) |
| **emitted-code differential** — example driver (full transform mode) over 11,999 real tsx/jsx (corpora A, B, and vscode's tsx), stdout+stderr+exit per file | **11,999 / 11,999 identical**; no file crashed either driver |
| synthetic large-state components (`gen_syn*.py`) | 12/12 identical (the largest, `cf_00400`, re-run sequentially after its baseline compile was killed by memory pressure in the parallel run: base 257 s vs patched 96 s, byte-identical output) |
| standalone clippy of the crate (1.96.0, default lints; the workspace build caps vendored-crate lints so this is otherwise invisible) | 0 errors; 82 pre-existing warnings vs 83 patched — one new `too_many_arguments` (the added `pool` parameter) |

No file in these corpora triggers the known unrelated `SIGABRT` ("Expected a node for all
scopes") in either binary.

## 3. Performance

### 3.1 Real corpora — end to end (native oxlint, rule alone)

Interleaved rounds (alternating which binary runs first, load-gated, 1 warm-up), corpus A,
`--disable-nested-config -c rc-only.json --format json`:

| threads | baseline median (min–max) | patched median (min–max) | paired wins | ratio |
|---|---|---|---|---|
| 16 (7 rounds) | 1,812 ms (1,729–2,011) | 1,554 ms (1,365–1,774) | **7/7** | 1.17× |
| 1 (3 rounds) | 21,657 ms (21,594–21,885) | 17,981 ms (17,150–20,166) | **3/3** | 1.20× |

Modest because **no file in ~26k real files (six large OSS React apps) is a multi-thousand-line
single component**: per-file single-thread timing of the ~150 largest tsx/jsx files
(`results/perfile*.tsv`) shows the worst real file, cal.com `bookings-single-view.tsx`
(~1,900 lines), at 798 ms → 383 ms (2.1×) whole-file, most large components at 1.1–2.6×.
The pathological "thousands of lines of TSX per component" shape the patch was built for is a
private-monorepo phenomenon; §3.3 reconstructs it.

### 3.2 The repo's criterion micro-bench (`--bench react_compiler`)

`TestFiles::minimal()` in transform mode, both trees, 2 interleaved rounds: **no measurable
difference** (App.tsx 8.39–8.49 ms both; kitchen-sink 684–694 µs both; Radix 1.78–1.89 ms
both; react.development.js / binder.ts sub-µs early-exit). None of these fixtures reaches the
pass meaningfully — excalidraw `App.tsx` is a *class* component (the compiler skips it; its
compiler-pass time is ~1 ms of the 8.4). This bench cannot see the change.

### 3.3 The pathology, reproduced — per-pass timers

Using the compiler's own `TimingData` (enabled via `measure-timing.patch`; sums over all
compiled functions in a file), single file, single thread. `cf_*` = a synthetic control-flow-
heavy function component (`gen_syn2.py`: per section — ternaries/`??`/`&&`, an `if/else if`
chain, a `useCallback` closure capturing locals, object spreads).

| file (lines) | pass `InferMutationAliasingEffects` base → patched | pass share (base) | all compiler passes base → patched |
|---|---|---|---|
| cf_00100 (1,809) | **9,398 ms → 317 ms (29.7×)** | 77.6 % | 12,107 → 2,875 ms (4.2×) |
| cf_00200 (3,609) | **37,264 ms → 1,210 ms (30.8×)** | 72.9 % | 51,129 → 14,611 ms (3.5×) |
| cal.com `bookings-single-view.tsx` | 646 ms → 29 ms (22×) | 68.9 % | 938 → 329 ms (2.9×) |
| posthog `Combobox.tsx` | 520 ms → 28 ms (19×) | 88.9 % | 585 → 89 ms (6.6×) |
| cal.com `Booker.tsx` | 134 ms → 8 ms (17×) | 74.1 % | 181 → 52 ms (3.5×) |
| posthog `SurveyEdit.tsx` | 61 ms → 4 ms (15×) | 40.4 % | 151 → 87 ms (1.7×) |

This matches the shape of the reference measurements the patch came with (pass 76–93 % of
compile on the worst files, ~25–30× on the pass): on a 3.6k-line component the pass goes from
73 % of compile time to 8 %, ~30× faster; on real (smaller) components it is 15–22× and its
share collapses from 40–89 % to single digits.

Whole-file ratios shrink as N grows (`results/sweep-controlflow-shape.tsv`: 3.7× at 1.8k lines,
3.4× at 3.6k, 2.4× at 7.2k) because once the pass collapses the residual is other passes the
patch does not touch — `codegen`, `InferReactivePlaces`, HIR `lower`, post-dominator
computation — some of which have their own super-linear growth. A JSX-children-heavy shape
(`gen_syn.py`, `results/sweep-jsx-shape.tsx`) is instead dominated by HIR lowering
(`lower_block_statement_inner`) in *both* binaries and shows only ~1.4×: the win is specific to
functions whose cost is the aliasing fixpoint.

## 4. Audit of the two scrutiny points

### (1) The merge "did this entry change?" test — equivalent

Old test (per value entry): `merged.kind != this.kind || !is_superset(this.reason,
merged.reason)`. Patched test: `merged_handle != this_handle` with `merged_handle =
pool.merge_values(this, other)`.

`ValuePool::merge_values(a, b)` returns handle `a` itself exactly when
`merge_value_kinds(a_k, b_k) == a_k == b_k && a.reasons ⊇ b.reasons` (the TS fast path), else
it interns `{ kind, reasons: a's sequence + b's genuinely new reasons }`. Case analysis:

| situation | old test | new test |
|---|---|---|
| merged kind ≠ `a.kind` | change | new value has a different kind → different handle → change |
| kind unchanged, `b`'s reasons ⊆ `a`'s | no change (`is_superset` true) | fast path returns `a`; or — when `b_kind ≠ kind` blocks the fast path — the built value is `(a.kind, exactly a's ordered reasons)`, which canonical interning maps back to handle `a` → no change |
| kind unchanged, `b` adds ≥1 reason | change | key `(kind, a's seq + new)` differs → different handle → change |
| set-equal but differently-ordered reason lists (distinct handles) | `is_superset` both ways → no change | fast path returns `a` (superset holds) → no change |

So the handle test coincides with the old structural test in every case. Interning is canonical:
`intern()` keys ≤1-reason values by an exact `(kind_index, reason_slot)` slot and multi-
reason values by `(kind_index, reasons in insertion order)`; `intern_sorted_set` keys sets by
sorted content, so equal sets always share a handle (the variables loop's `has_new` scan ⇔
`set_union(a, b) != a`). The TS reference confirms the framing: TS's own change test is
object identity (`mergedValue !== thisValue`) with `mergeAbstractValues` returning `a` in the
no-change case — canonical handle equality is its direct analogue, and the port had already
replaced identity with the structural test that the patch preserves.

Sharing safety: every write into the pool is an append (`values.push`, `sets.push`, new-key
inserts); `merge_values` clones a value's reasons into a *local* before interning; the state
maps only `insert` fresh handles. Nothing reachable through a handle is mutated in place.

`queue()`: TS re-`set`s the *same* queued object when `merge()` returns null (a no-op that
preserves the Map slot), so dropping the port's clone-and-reinsert is faithful; the queued
map's order is not iterated (the driver walks `func.body.blocks.keys()`); successor queue
order is unchanged (all-but-last in order, then the last, moved).

### (2) The value-set iteration order — the one empirical point

`kind()` / `kind_opt()` fold `merge_values` over a variable's value set; old iteration order
was `FxHashSet<ValueId>` order, new is ascending `ValueId`. The fold order determines the
merged value's reason *sequence*, which reaches exactly one order-sensitive consumer:
`primary_reason()` (first non-`Other` reason), used at two sites — the `reason` field of the
`Create` effect emitted in the `CreateFrom` copy path. Everything else touching reasons is
order-insensitive (`get_write_error_reason` is a priority chain of `.contains()` checks;
`is_superset` is set-based). No downstream pass reads `Create.reason` (ranges uses only `into`;
analyse_functions treats `Create` as a no-op; `AbstractValue` never leaves the file); the one
live path is *within* this pass — a nested function's recorded effects re-applied at an outer
`Apply` reach `apply_effect`'s `Create` arm (`intern_simple(kind, reason)`), so a
`primary_reason` chosen inside a nested function can seed an outer value's reason and, if that
value is later illegally mutated, select the hint text.

The old order on that path was already nondeterministic run-to-run (`ValueId` is a
process-global atomic counter → hash order shifts, especially across worker threads), so
shipped 1.71.0 cannot systematically depend on it. Ascending `ValueId` is *allocation order* —
stable across runs and threads (relative order within one function is fixed even though
absolute ids shift), i.e. the patch replaces a nondeterministic order with a deterministic
one closer to TS's insertion-ordered `Set`. Empirically identical on ~26k files including the
`--threads 1` probe that removes the old nondeterminism and so directly compares the two
orders. As stated, this is empirical for arbitrary inputs, not structural.

On the "keep insertion order in the pooled set" idea: it would match TS but not the shipped
port (whose order was hash order, unreproducible), so it buys TS-fidelity, not
1.71.0-identity; it is implementable canonically (union = "a followed by b's new elements") at
similar cost. Recommendation: keep ascending `ValueId` and state the argument as above.

### Enum index tables

`value_kind_index` / `value_reason_index` match `type_config.rs` exactly (6 and 12 variants,
declared order). One correction: a variant *reordering* would **not** alias fast-path slots —
the tables are exhaustive `match`es on variant *names*, so declaration order is irrelevant
(unlike a `reason as u16` bitmask, which is order-dependent). The real maintenance hazard is
the hand-set `VALUE_KIND_COUNT` / `VALUE_REASON_COUNT`: adding a variant is a compile error in
the match, but nothing forces bumping the count, and an un-bumped count would collide the new
reason's slot with the empty-reason slot. Cheap fix: derive the counts, or assert
`index < COUNT`.

## 5. Reviewer nits

- Three added lines are 101 columns; one stray blank line before `impl ValuePool`'s closing
  brace. (The pristine crates.io file is itself ~1,500 lines away from rustfmt 1.96.0's opinion
  under both oxc's and the fork's configs, so whole-file `rustfmt` is not the yardstick here.)
- `*_COUNT` constants hazard above.
- One new `clippy::too_many_arguments` warning (crate default lints; 82 pre-existing).
- `merge_cache` / `union_cache` are unbounded pair-keyed memos with a theoretical O(V²) bound;
  they stay small on real functions — worth a sentence next to "the pool only grows … and is
  released when the pass finishes" (which already covers the transient-memory point).

## 6. Upstream verdict

- **`oxc-project/forked-react-compiler`: not worth it as things stand.** It is a mechanical
  mirror of `facebook/react`'s compiler tree (`just sync` re-extracts and re-applies
  `patches/`); hand edits to the vendored Rust would live only as a patch-stack entry, and oxc
  no longer consumes the crates. The maintainers' own precedent (`patches/0004-fx-hashers…`,
  later upstreamed to react as #36811) is: carry as a patch, upstream to react.
- **`facebook/react` `compiler/crates` (the origin): the fix is genuinely novel there** —
  `main` still deep-copies both maps and clones twice per block visit. That would be a Meta
  contribution (CLA, their review) — a decision for the humans; nothing was opened or contacted.
- **`oxc-project/oxc` `main`: superseded in intent.** `#24117` already removed the per-entry
  deep copies (`Rc`-shared value sets) and `#24480` collapsed reason sets to a `u16` bitmask,
  so the interning delta over main is handle-compare + memoized merges only. Whether that
  measurably beats main's representation is a separate question against main's diverged file
  (this patch does not apply to it).

For a repository running shipped oxlint 1.71.0, the patch as validated here is a sound
backport: identical diagnostics and emitted code across everything measured, ~30× on the
pathological pass, 15–22× on the pass for real large components.
