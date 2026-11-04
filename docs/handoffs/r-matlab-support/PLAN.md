# Plan: R + MATLAB language support for `pmat context` (tree-sitter)

- **Date:** 2026-10-23 (v2, revised after the quantum-lens review; v1 is superseded)
- **Project:** paiml-mcp-agent-toolkit, the user's **own fork** (pmat 3.42.0, HEAD 659f6ae6b)
- **Status:** Phase 0 DONE 2026-10-28 → MATLAB **GO** (535/535 exact vs mtree, 0% span-ERROR). See `pmat-r-matlab-language-support/spike-report.md`. Next: Phase 1.
- **Decided by user:** "tree-splitter" = tree-sitter. The user adds the `tree-sitter-r` /
  `tree-sitter-matlab` deps to `Cargo.toml` themselves. The work targets a fork, not an upstream PR.
- **Review record:** `.quantum-lens/outputs/analyses/2026-10-23-plan-r-matlab-tree-sitter-support-for-pmat-conte-e99d8803.md`
  (score 7.7)

## What changed from v1, and why
| v1 | v2 | Reason |
|---|---|---|
| Wiring = `detect_language` + `dispatch.rs` | **5-gate funnel:** discover → detect → dispatch → parse → report | `src/services/file_discovery_filters.rs:100-145` allowlists `m`/`mm` but **not `r`**, so all 180 R files are dropped before detection. A correct R parser would still show 0 items |
| Baseline "0 R/MATLAB items" | **Per-gate funnel counts** per file | v1's zero is true for two different reasons and can't tell them apart |
| MATLAB ground truth = `grep ^\s*function` | **MATLAB R2023b `mtree` oracle** (test-time only), graded on **disagreements** | grep *is* the regex option the plan rejects, so a 95% gate against it rewards copying regex's blind spots |
| K1 = any ERROR node in >5% of files | K1 = **ERROR inside ≥5% of function spans, OR name/start-line mismatch vs oracle in ≥5%** | Per-file "any ERROR" is too strict (362 command-syntax lines) and too weak (a wrong-but-valid parse has no ERROR node) |
| Phase 0 = 2h toolchain, then grammar spike | **Phase 0 = grammar spike in Python, no cargo** | PyPI has `tree-sitter` 0.26.0 + `tree-sitter-matlab` 1.3.1 win_amd64 wheels. Risk is measured before cost is sunk |
| Phase 3 = `.m` content sniffer (2h) | **`.m` → MATLAB by default** (≈15 min) | The context path's `detect_language` has no ObjC arm, and the corpus has 0 ObjC files. A sniffer's main risk is misrouting function-less scripts into a silent zero |
| Hand-written visitor (A), with tags.scm optional | **tags.scm for extraction + a small complexity walker**, if the spike passes | A visitor duplicates what the grammar's `queries/tags.scm` already gives |
| Regex fallback when features are off | **Grammars default-on in the fork; no regex fallback** | The fork has one user, so the fallback is dead code |
| Script = "file-level item" | **`%%` cell items for scripts**; classdef stubbed (methods only) | Census below: 75 scripts hold 1,015 `%%` cells. Only 2 classdef files |

## Corpus census (measured 2026-10-23, `mim_yaoa_conn_slide`)
- **MATLAB, 261 files:** 184 function files, 75 scripts, 2 classdef. 538 `function` defs, 1,015 `%%` cells,
  362 command-syntax lines (`hold on`, `clear`, `addpath`, …).
- **R, 179 files:** **669 function bindings** per the getParseData oracle. The regex census said 889, but it counted `tryCatch(error = function…)`/`FUN = function` named args. No `.Rmd`/`.qmd`/`.mlx`.
- Script: `docs/handoffs/r-matlab-support/spike/census.py` (re-run it on any other target repo).

## End State
`pmat context` on `mim_yaoa_conn_slide` covers all of the following:
- every R function def (with exact name and start line);
- every MATLAB function, including local and nested ones;
- a `%%` section item for each MATLAB script cell;
- a cyclomatic/cognitive complexity for each function.

`pmat query` indexes them. The funnel report shows 180/180 R and 261/261 `.m` files passing every
gate. `pmat verify` is green on the fork branch.

## Success Criteria
- **Funnel:** 100% of R and `.m` files are discovered, detected and dispatched.
- **R:** ≥98% of the 669 `getParseData` function bindings are matched on name + start line, over the whole corpus (not a sample).
- **MATLAB:** ≥95% of the 535 `mtree` definitions (Phase 0 spike: 100%) are matched on name + start line, over the whole corpus. Every mismatch is listed and
  triaged.
- Tests in `cargo test --lib`, and `pmat verify --format json` returns `ok:true`.
- **Not in scope:** comply/mutation/TDG/dead-code/WASM, `.Rmd`/`.mlx`, unifying the 11 `Language` enums,
  full classdef property modelling, ObjC `.m` support, upstream docs/PR.

**Kill / reassess criteria:**
- **K1 (Phase 0):** MATLAB name/line mismatch vs mtree ≥5% of 538 defs, OR ERROR inside ≥5% of function
  spans. If triggered, drop tree-sitter for MATLAB, then pick regex graded by mtree or R-only scope.
- **K2:** toolchain setup takes >2h. If triggered, switch to WSL2 or Codespaces.
- **K3:** binary headroom is measured in Phase 1. If the grammars exceed it, delete or relax
  `src/tests/binary_size.rs:40` in the fork (record the decision) rather than contorting features.
- **K4:** more than 15 compile errors from adding the enum variants (CRUX 2, Phase 2). If triggered, use string-keyed
  dispatch only (no new enum variants) and stop extending enums.
- **K5:** more than 25h in total (estimate ~15h). If triggered, reassess.

## Assumptions
| # | Assumption | Validation | If wrong |
|---|---|---|---|
| A1 | `mtree` (undocumented, ships with R2023b) exposes function name + line | Phase 0, first 5 min | Use `checkcode -struct`/`matlab.internal.getCode` or hand-label 30 files |
| A2 | `tags.scm` in both grammars marks function definitions with name nodes | Phase 0 | Write a node-kind walker (dump kinds in Phase 0) |
| A3 | ❌ FALSE: no wheel; R grammar check moved to Phase 3 | A `tree-sitter-r` Python wheel is installable | Phase 0 (`pip install tree-sitter-r`) | Do the R spike in Phase 3 with cargo (R grammar is lower risk) |
| A4 | Both grammar crates build with `tree-sitter` 0.26 (tsl 0.1.7) on MSVC | Phase 1 | Pin a grammar version, or vendor `parser.c` |
| A5 | The only other gate is the discovery filter. No other allowlist drops `.r`/`.m` | Phase 1 funnel trace | Add the gate to the funnel, then fix it |

## Phases

**Phase 0: Grammar spike, no cargo (1–1.5h). Answers the Killer Question**
- Scope: a Python venv in the scratchpad with `tree-sitter` + `tree-sitter-matlab` (+ `tree-sitter-r` if
  available). Write a MATLAB `mtree` oracle script (`fun_name`, `start_line` per file → CSV) and an R
  `getParseData` oracle (`Rscript`). Write a Python extractor that uses each grammar's `tags.scm` and
  prints ERROR spans. Diff the extractor output against the oracles over the **whole** corpus.
- Deliverable: `spike-report.md` with the mismatch %, the list of span-ERROR defs, the mismatch list,
  the node kinds for function/class/`%%` comments, and a tags-vs-walker verdict.
- Gate: K1 evaluated. GO / NO-GO recorded.

**Phase 1: Toolchain + funnel baseline (1.5–2h)**
- Scope: ~~rustup (MSVC host)~~ **done with MSYS2 MINGW64 instead (see HANDOFF.md)**, `cargo build --release`
  on the branch `add_code_analysis_support_MATLAB-R`. Record the binary size against the band in
  `tests/modules/binary_size_band.rs` (the 52,428,800 cap cited earlier is a dead path). Write a funnel
  probe, either a throwaway `#[test]` or a debug log that counts files per gate for the target repo.
- Deliverable: a baseline funnel table (expected: R stops at **discover**, `.m` stops at **detect**)
  and the headroom number.
- Gate: the build passes, the funnel table exists, and K2/K3 are evaluated.

**Phase 2: Discovery + detection + enum probe (1–1.5h)**
- Scope: add `"r"` to `file_discovery_filters.rs:100-145` (the extension is lowercased there). In
  `deep_context/analysis_functions/metrics_complexity.rs:1` map `r`/`R` → `"r"` and `m` → `"matlab"`.
  Add `normalize_language_name` aliases in `language_override.rs:64`. Append `R`, `Matlab` to
  `ast/core/types.rs` (after `Scala = 18`) and to `semantic/chunker_types.rs`. Count the compile errors
  that follow (CRUX 2, K4).
- Deliverable: a funnel showing 100% of files reach **dispatch** (still 0 items), and the enum-site count.
- Gate: the funnel reaches dispatch, existing `language_registry_tests`/`language_override` tests pass, and K4 is evaluated.

**Phase 3: R extraction + complexity (2–3h)**
- Scope: `src/ast/languages/r.rs` using `tree-sitter-r` + a tags query. Extract `<-`/`=`/`<<-` function
  bindings, `setGeneric`/`setMethod`/`R6Class` names, and `library`/`source` imports. Complexity walker:
  `if`/`for`/`while`/`repeat`/`switch`/`&&`/`||`. Unit tests on inline fixtures.
- Gate: tests pass, and ≥98% match vs `r_oracle.csv` (669 bindings) (mismatch list attached).

**Phase 4: MATLAB extraction + complexity (2–3h)**
- Scope: `src/ast/languages/matlab.rs`. Extract functions (primary/local/nested), `%%` cells in scripts as
  section items, classdef → class + method names only. Complexity:
  `if`/`elseif`/`for`/`parfor`/`while`/`case`/`catch`/`&&`/`||`. Fixtures must include command
  syntax, `'` transpose, `end` in indexing, and `end`-less function files.
- Gate: tests pass, and ≥95% match vs `m_oracle.csv` (535 defs), with every mismatch triaged. ALSO (from spike): a fixture that reproduces the unclosed top-level script `for`/`parfor` ERROR (`mim_prep_hpg_amica.m`, `spca_e_psd_gen.m`), and a keyword/`%%` text fallback scoped to ERROR spans, so script complexity doesn't undercount (1.7% of lines, ~28 cells).

**Phase 5: Wire into `pmat context` (1.5h)**
- Scope: arms in `dispatch.rs:45` and `metrics_complexity.rs:120`, plus `analyze_r_language`/`analyze_matlab_language`
  (pattern: `scripting.rs:112`). Make the features part of `default` in the fork's Cargo.toml.
- Gate: re-run the funnel. Every gate is 100% and item counts equal the Phase 3/4 extractor counts.
  Save the before/after `pmat context` diff.

**Phase 6: Semantic chunker / `pmat query` (1.5–2h)**
- Scope: `chunk_r_file`/`chunk_matlab_file` (pattern: `semantic/chunker_go_lua.rs`), `chunker_extract.rs`,
  `agent_context/function_index/build.rs`.
- Gate: after an index rebuild, `pmat query "<known R fn>"` and `pmat query "<known MATLAB fn>"` return the correct file:line.

**Phase 7: Harden + fork hygiene (1h)**
- Scope: `pmat verify --format json`. Write a `FORK.md` note listing the touched upstream files, for rebase
  cost. (`.quantum-lens/`, `.mcp.json`, `.claude/` are ignored via `.git/info/exclude`, local only —
  re-add them there on a new clone.)
- Gate: `ok:true`, and all tests green.

**Estimate:** ~13–16h. The extraction work in Phases 3–4 is where most of the risk sits once Phase 0
has passed.

## Verification
- **Works:** a whole-corpus diff against independent oracles (getParseData, mtree), not a sample, and not
  grep. Funnel counts at each gate. Unit fixtures for MATLAB's hard syntax.
- **Keeps working:** keep the oracle CSVs and a small fixture subset in `tests/fixtures/{r,matlab}/`. An
  integration test asserts the extracted name+line set against the stored oracle CSV.

## Open questions
1. If Phase 0 shows the `tree-sitter-r` wheel is missing, is R-via-cargo in Phase 3 acceptable? (Default: yes.)
2. Do you want deeper classdef modelling (properties/events) later? It is stubbed now (2 files).
