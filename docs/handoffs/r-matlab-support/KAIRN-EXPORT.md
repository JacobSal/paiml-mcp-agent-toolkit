# Kairn export — pmat R + MATLAB support

Exported 2026-11-04 from the Kairn instance on the original Windows machine.
Source project id there: `d6ba2a0f` (ids are instance-local; do NOT reuse them on import).

## How to import on another machine

Ask Claude Code, in the repo on the new machine:

> Read `docs/handoffs/r-matlab-support/KAIRN-EXPORT.md` and replay it into Kairn:
> create the project, then the three log entries, then the five learnings, exactly as written.

Each block below maps 1:1 to a Kairn MCP call. Field names are the tool parameters.

---

## 1. Project — `kn_project`

Create first (phase cannot be set on create), then call again with `project_id` + `phase: active`.

- **name:** `paiml-mcp-agent-toolkit (pmat R + MATLAB support)`
- **phase:** `active`
- **goals:**
  - Add R and MATLAB code analysis to pmat via tree-sitter (tree-sitter-r 1.3.0, tree-sitter-matlab 1.3.1) behind r-ast / matlab-ast features
  - Fork github.com/JacobSal/paiml-mcp-agent-toolkit, branch add_code_analysis_support_MATLAB-R; no upstream PR until grammar wiring (Phases 2-4) lands
- **success_metrics:**
  - P3: R definitions >=98% vs getParseData oracle (669 bindings)
  - P4: MATLAB definitions >=95% vs mtree oracle (535 defs) in Rust
  - P7: pmat verify --format json ok:true
  - Binary within tests/modules/binary_size_band.rs band (55,000,000 B, quiet +-5%, fail +-20%)

## 2. Progress log — `kn_log` (type `progress`, in this order)

### 2a
- **action:** Phase 0 spike (2026-10-23..28): py tree-sitter 0.26 + tree-sitter-matlab 1.3.1 + its tags.scm vs MATLAB R2023b mtree oracle on mim_yaoa_conn_slide -> 535/535 exact name+line, 0% span ERROR -> MATLAB GO. R oracle via getParseData = 669 bindings (regex 889 overcounted). Plan v2 after quantum-lens review (7.7).
- **result:** MATLAB GO; R grammar to be checked through cargo in Phase 3. Artifacts in docs/handoffs/r-matlab-support/ on the branch.

### 2b
- **action:** Phase 1 (2026-10-29..11-04): toolchain = MSYS2 MINGW64 (rust 1.99.1 windows-gnu, gcc 16.2). Split unrelated ROADMAP/POLYGLOT edits into own commit; .gitignore lines -> .git/info/exclude. Cargo.toml: optional tree-sitter-r/matlab + r-ast/matlab-ast features in all-languages. Release build OK 36m40s, 57,288,704 B (in quiet band; grammars linker-stripped until referenced). Windows test fixes: ExitStatusExt split per platform in 2 test modules, 2 symlink/chmod tests cfg(unix), clone_on_copy allow. pmat verify: format/complexity/satd/clippy green; tests blocked by aprender-viz cdylib export-ordinal limit -> local .cargo/config.toml opt-level=2 for deps (dev+test).
- **next_step:** see HANDOFF.md "Next steps" (funnel probe, then Phase 2).

### 2c
- **action:** Final Windows gate 2026-11-04. `pmat verify`: format, complexity, satd and clippy green; tests 21,728 passed / 83 failed / 151 ignored (a re-run gave 90 failed, a flaky set). All failures are Windows-environment issues: `\` paths, `\\?\` canonical paths and 8.3 names, missing bash, `/proc`, file locking, case-insensitive FS, CRLF in generated-file comparisons, a tokio current-thread `block_in_place`. None are caused by the branch. Evidence: the TDG-baseline failure was a stale cache (the same tree passes on re-run), and the 2 failures in edited files are in tests the diff does not touch. Ledger in `windows-test-failures.md`. HANDOFF.md written; commits split 5 ways; the branch needs a force-with-lease push (it was rewritten).
- **result:** Windows cannot give `ok:true`. The green gate must be run on Linux before Phase 2 is committed. The next step is a Linux `pmat verify`, then the funnel probe, then Phase 2.

## 3. Learnings — `kn_learn` (confidence `high`, namespace `knowledge`)

### 3a — type `gotcha`, tags `pmat, msys2, rust, windows, toolchain`
On this Windows machine the Claude Code Bash tool is Git for Windows, whose /usr/bin/pacman is a non-working stub (/etc/pacman.conf missing). Real MSYS2 is C:/msys64; Rust/cargo/clippy/rustfmt 1.99.1 + gcc 16.2 live in its MINGW64 env. Use PATH=/c/msys64/mingw64/bin:$PATH; target is x86_64-pc-windows-gnu, so cfg(unix) is FALSE (no MSYS2 env gives a supported unix Rust target).

### 3b — type `gotcha`, tags `pmat, rust, windows-gnu, linker, cargo-test`
pmat `cargo test` on x86_64-pc-windows-gnu fails to link dependency aprender-viz (lib trueno_viz, crate-type cdylib+rlib): "ld.exe: export ordinal too large: 78297" (PE cap 65,535). Cause: opt-level 0 share-generics re-exports dependency generics. Per-crate opt-level on aprender-viz alone only reduced it to 77,692. Fix: opt-level=2 for ALL deps, and it must be set for [profile.test.package."*"] too, because pmat's Cargo.toml pins that to 0 and cargo test uses the test profile. Put it in an untracked .cargo/config.toml (config overrides manifest) listed in .git/info/exclude. Clippy and release builds never hit it.

### 3c — type `gotcha`, tags `pmat, binary-size, kill-criteria`
pmat size gate: the real binary-size test is tests/modules/binary_size_band.rs (EXPECTED_BYTES 55,000,000; quiet +-5%; fail +-20%). CLAUDE.md's src/tests/binary_size.rs / 52,428,800 cap is a dead path. Unreferenced tree-sitter grammars are linker-stripped, so the size cost of tree-sitter-r/matlab only appears once code calls them (re-measure after Phase 2-4; ~460 KB headroom to the quiet-band warning at 57,750,000).

### 3d — type `decision`, tags `pmat, cargo-features, tests, windows`
pmat R/MATLAB grammars are optional deps behind features r-ast = ["tree-sitter","tree-sitter-r"] and matlab-ast = ["tree-sitter","tree-sitter-matlab"] (lua-ast pattern), listed in all-languages only, NOT core-languages/default until the K3 size measurement after wiring. Windows-only test failures are fixed by gating the narrowest unit (import + helper per platform), never whole test modules: whole-module gating silently dropped 46 pure-logic tests.

### 3e — type `gotcha`, tags `pmat, windows, cargo-test, verify`
pmat `cargo test --lib` cannot be green on Windows (x86_64-pc-windows-gnu). About 83–90 of 21,962 tests fail for environment reasons, with the full test stage taking ~78 min:
- `\` and `\\?\` paths and 8.3 short names compared as strings;
- bash/sh missing;
- `/proc`;
- SQLite file locks;
- a case-insensitive FS;
- CRLF/paths in generated-file comparisons.

Run the authoritative `pmat verify` on Linux. Watch out for the TDG-baseline test (`the_committed_baseline_is_the_measured_count`): it can fail from a STALE CACHE after a full-suite run. Re-run it alone before blaming a diff. Files rewritten by `git checkout` refresh the cache and make it pass.

---

## Not in Kairn (lives elsewhere)

- **Evolving Lite project memory:** `evolving-lite-project.json` in this directory is a verbatim copy of
  `primeline-ai-reorg/plugins/evolving-lite/_memory/projects/paiml-mcp-agent-toolkit.json`. Drop it into the
  same path on the new machine and set `active_project` to `paiml-mcp-agent-toolkit` in `_memory/index.json`.
