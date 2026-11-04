# Phase 0 spike report: grammar quality without cargo

- **Date:** 2026-10-28
- **Corpus:** `mim_yaoa_conn_slide` (261 `.m` and 179 `.R` files tracked by git)
- **Verdict:** **MATLAB GO** (K1 not triggered). **R oracle ready**; the R grammar check is deferred to Phase 3, because `tree-sitter-r` has no PyPI wheel (A3 false).

## Setup
- Python 3.12 venv with `tree-sitter` 0.26.0 and `tree-sitter-matlab` 1.3.1 (win_amd64 wheels).
- Extraction uses the grammar's own `queries/neovim/tags.scm` (`@definition.function`, `@definition.class`). No hand-written visitor.
- **MATLAB oracle:** R2023b `mtree` (`mtfind(t,'Kind','FUNCTION'|'CLASSDEF')`, `Fname`/`lineno`) → `m_oracle.csv`. It runs over 261 files in 13.6 s.
- **R oracle:** R 4.6.1 `getParseData` → FUNCTION token → enclosing `LEFT_ASSIGN`/`EQ_ASSIGN`/`RIGHT_ASSIGN` expr → SYMBOL/STR_CONST target → `r_oracle.csv`.

## MATLAB results (`m_compare.txt`)
| Metric | Value |
|---|---|
| Oracle definitions (parseable files) | 535 (533 function + 2 classdef) |
| tree-sitter definitions | 535 |
| **Exact name + start-line match** | **535 / 535 (100.00%)** |
| Missed / extra | 0 / 0 |
| **Span-level ERROR/MISSING (v2 K1)** | **0 / 535 (0.00%)** |
| File-level any-ERROR (v1 K1) | 23 / 261 (8.81%). **v1 would have falsely KILLED the project** |
| Lines inside ERROR spans | 1,324 / 76,062 (1.7%) |
| `%%` cells recovered as `comment` nodes | 987 / 1,015 (97.2%) |

- **Property accessors.** The grammar emits a single `get.`/`set.` token before the identifier, and the extractor rebuilds `get.Prop`. With that, all 26 accessors in `Violin.m` match.
- **Files `mtree` itself rejects (4, excluded from the denominator):** `mim_ls_merge_eeg.m` and `mim_merge_liveamps.m` contain real syntax errors (`error('...',)`, `f(,x)`). tree-sitter still recovers 1 and 4 definitions there. `fem_norm_dipfit.m` and `e_speed_anl_plots.m` are scripts.
- **Where the ERROR nodes are.** 20 of 26 sit under `source_file`: a top-level `for`/`parfor`/`if` block in a *script* that the grammar fails to close and resynchronises after. They are concentrated in a few files (`mim_prep_hpg_amica.m` 371/396 lines, `spca_e_psd_gen.m` 302/480, `spca_sbs_e_psd_gen.m` 219/385). The exact trigger was **not isolated**: a naive prefix bisect is confounded by nested `end`s. This is Phase 4 fixture work.
- **Impact on later phases.** Function extraction is unaffected. **Script complexity will undercount** inside those ERROR spans, and ~28 `%%` cells are lost. Phase 4 needs (a) a fixture that reproduces the trigger, and (b) a fallback that counts control-flow keywords and recovers `%%` lines textually *inside ERROR spans only*.

## R results
| Metric | Value |
|---|---|
| Files parsed by R | 179 / 179 (0 parse errors) |
| **Oracle function bindings** | **669** |
| Regex census (`(<-\|=) function(`) | 889. **It overcounts by 220.** The extra 187+ are `tryCatch(error = function(e))`, `FUN = function` and other named arguments (`EQ_SUB`, not bindings), plus 10 commented-out lines |

- The regex overcount confirms the review's point: a regex oracle would have set the wrong target, 889 instead of 669.
- Remaining Phase 3 task: build `tree-sitter-r` via cargo, then diff against `r_oracle.csv` (gate ≥98%).

## Assumptions resolved
| # | Status |
|---|---|
| A1 `mtree` exposes name + line | ✅ |
| A2 `tags.scm` marks function defs | ✅ (MATLAB). R is checked in Phase 3 |
| A3 `tree-sitter-r` wheel exists | ❌. The R grammar check moves to Phase 3 (cargo) |

## Artifacts (this directory)
`ts_extract.py`, `compare.py`, `mtree_oracle.m`, `r_oracle.R`, `census.py`, `m_files.txt`, `r_files.txt`,
`m_oracle.csv`, `m_ts.csv`, `m_compare.txt`, `m_errors.txt`, `r_oracle.csv`.
Re-run the MATLAB diff with:
`python -I compare.py m_oracle.csv m_ts.csv`
(this needs a venv with `tree-sitter==0.26.0 tree-sitter-matlab==1.3.1`).
