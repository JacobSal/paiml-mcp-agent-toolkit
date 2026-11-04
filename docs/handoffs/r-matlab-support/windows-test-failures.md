# `cargo test --lib` failures on Windows (x86_64-pc-windows-gnu, MSYS2 MINGW64)

Measured 2026-11-04 on this branch (uncommitted work as of the Phase 1 commits), by running the
already-built test binary directly:
`RUST_MIN_STACK=8388608 target/debug/deps/pmat-<hash>.exe -q`.

- **Run 1** (inside `pmat verify`): 21,728 passed, **83 failed**, 151 ignored; 4,653 s.
- **Run 2** (this table): **90 failed**. The difference between the runs is flakiness from shared temp
  and cache state, and from a 1,800 s measurement timeout.

## Verdict: none of these are caused by this branch

The branch changes only three things: Cargo features that are off by default, five test files,
and docs. Two checks back the verdict:

1. **The TDG baseline failure** (`the_committed_baseline_is_the_measured_count`, 1704 vs 1680)
   comes from a stale cache. The same binary on the same tree passed twice on re-runs.
2. **The two failures in files this branch edits** are in tests the diff does not touch:
   - `cb1664_green_on_valid_absolute_command`: a Windows path with `\` is written raw into a JSON string,
     which fails with "invalid escape".
   - `test_resolve_spec_path_gh_prefix`: the filesystem is case-insensitive.

## Causes, by group

| Cause | Examples |
|---|---|
| `\` separators, `\?\` canonical paths and 8.3 short names (`<USER~1>`) compared as strings | dead_code_handlers `*_root/subtree/budget`, `cargo_dead_code_analyzer`, `agent_context`, `vector_store_scoping`, `work_agent_claims` |
| `bash`/`sh` hook scripts and shells that are missing (os error 3) | `hook_clippy_gate_tests` x5, `rust_project_score::command_runner` x4 |
| Unix-only process and system facts: `/proc/self/status`, pipe inheritance, env depth | `metrics_ratchet::*`, `throughput_verdict_tests`, `test_env_hygiene` |
| Windows file locking and permissions: open SQLite file (os 32), aging an mtime (os 5) | `tdg::storage_backend` x2, `pmat_cache_dir::tests_sweep` |
| Case-insensitive filesystem, so `.idea`/`.vscode`/`Thumbs.db` probes and `GH-42` resolve differently | `hygiene_scorer` x9, `qa_work_handler::impl_spec_tests` |
| Generated-file comparisons that differ because of line endings or paths (`mcp.json`, `.agents/hooks.json`, unrun-tests ledger) | `tool_manifest`, `check_macs`, `unrun_tests` |
| tokio `block_in_place` in a current-thread runtime, at `quality_checks_part2_security_duplicates.rs:162` (unclear why it only shows here) | `quality_gate_*` x10 |
| Repo-wide ratchet measurements that time out or read stale cache | `.pmat-ratchet.toml is red at HEAD`, `tdg_baseline` |

**Consequence:** this machine cannot produce `pmat verify` `ok:true`. To get the real green gate, run
`pmat verify --format json` on Linux (WSL2, CI or the next machine) before opening the upstream PR.

## Full list (run 2)

| Test | Panic location | First line of message |
|---|---|---|
| `cli::analysis_utilities::quality_gate_suite_tests::a_check_with_nothing_to_read_is_not_reported_as_run` | `src\cli\analysis_utilities\quality_checks_part2_security_duplicates.rs:162:31` | can call blocking only when running on the multi-threaded runtime |
| `cli::analysis_utilities::quality_gate_verdict_parity_tests::test_cli_and_mcp_quality_gate_agree_on_one_corpus` | `src\cli\analysis_utilities\quality_checks_part2_security_duplicates.rs:162:31` | can call blocking only when running on the multi-threaded runtime |
| `cli::command_dispatcher::contract_query_handlers::contract_query_handlers_tests::test_collect_rs_files_recurses_into_subdirs` | `src\cli\command_dispatcher\contract_query_handlers.rs:232:9` | assertion `left == right` failed |
| `cli::command_dispatcher::semantic_commands::vector_store_scoping_tests::test_default_store_is_per_workspace` | `src\cli\command_dispatcher\semantic_commands.rs:196:9` | /home/u/projects/alpha\.pmat\embeddings.db |
| `cli::command_dispatcher::semantic_commands::vector_store_scoping_tests::test_default_store_lives_under_dot_pmat` | `src\cli\command_dispatcher\semantic_commands.rs:205:9` | assertion `left == right` failed |
| `cli::handlers::comply_cb_detect::tests::cb081_dependency_tests::test_cb402_target_dir_excluded` | `src\cli\handlers\comply_cb_detect\tests_cb081_dependencies.rs:361:5` | Scripts in target/ should be ignored |
| `cli::handlers::comply_cb_detect::tests::cb900_markdown_tests::test_cb900_gates_exclude_paths_skips_vendored_docs` | `src\cli\handlers\comply_cb_detect\tests_cb900_markdown.rs:200:5` | assertion failed: violations.is_empty() |
| `cli::handlers::comply_cb_detect::tests::cb900_markdown_tests::test_cb900_pmat_yaml_file_health_exclude_skips_vendored_docs` | `src\cli\handlers\comply_cb_detect\tests_cb900_markdown.rs:222:5` | assertion failed: violations.is_empty() |
| `cli::handlers::comply_cb_detect::tests::cb950_yaml_tests::test_cb952_detects_missing_gha_fields` | `src\cli\handlers\comply_cb_detect\tests_cb950_yaml.rs:74:5` | assertion failed: violations.len() >= 2 |
| `cli::handlers::comply_cb_detect::tests::cb950_yaml_tests::test_cb953_detects_unpinned_action` | `src\cli\handlers\comply_cb_detect\tests_cb950_yaml.rs:102:5` | assertion `left == right` failed |
| `cli::handlers::comply_handlers::check_handlers::check_cot_proof::tests_cot_schema::derived_yaml_sanitizes_special_chars_in_id` | `src\cli\handlers\comply_handlers\check_handlers\check_cot_proof_tests_schema.rs:14:39` | called `Result::unwrap()` on an `Err` value: Os { code: 267, kind: NotADirectory, message: "The directory name is invalid." } |
| `cli::handlers::comply_handlers::check_handlers::check_macs::tests_macs_agy::cb1664_green_on_valid_absolute_command` | `src\cli\handlers\comply_handlers\check_handlers\check_macs_tests_agy.rs:169:9` | assertion `left == right` failed: .agents/hooks.json is not valid JSON (line 7 column 51): invalid escape at line 7 column 51 |
| `cli::handlers::dead_code_handlers::budget_tests::a_file_path_reports_that_files_findings_and_says_where_it_looked` | `src\cli\handlers\dead_code_handlers_budget_tests.rs:867:5` | assertion `left == right` failed: the listed path must be the file that was asked about: {"status":"completed","message":"Dead code analysis completed… |
| `cli::handlers::dead_code_handlers::subtree_scope_tests::a_lib_crates_subdirectory_reports_the_dead_code_the_crate_reports` | `src\cli\handlers\dead_code_subtree_scope_tests.rs:155:5` | the fixture must have dead code in the subtree for this to prove anything; the whole-crate run found [("src\\inner\\mod.rs", "NeverConstructed"), ("sr… |
| `cli::handlers::dead_code_handlers::subtree_scope_tests::a_bin_crates_subdirectory_reports_its_own_dead_code_and_no_one_elses` | `src\cli\handlers\dead_code_subtree_scope_tests.rs:200:5` | [("src\\inner\\mod.rs", "NeverConstructed"), ("src\\inner\\mod.rs", "dead_private_helper"), ("src\\other.rs", "outside_subtree_dead")] |
| `cli::handlers::dead_code_handlers::workspace_root_tests::a_path_inside_no_package_of_a_workspace_is_refused_not_reported_as_clean` | `src\cli\handlers\dead_code_workspace_root_tests.rs:179:5` | the refusal must name the manifest it stopped at, so the reader can check the claim: C:\Users\<USER~1>\AppData\Local\Temp\dcwsWc6pHJ\tools holds Rust … |
| `cli::handlers::dead_code_handlers::workspace_root_tests::a_workspace_manifest_yields_no_verdict_about_a_crate_it_does_not_declare` | `src\cli\handlers\dead_code_workspace_root_tests.rs:269:5` | the detail must name the manifest the verdict was taken from: LibraryTargetReport { verdict: "undetermined", detail: "cargo: the nearest Cargo.toml ab… |
| `cli::handlers::dead_code_handlers::subtree_scope_tests::the_whole_lib_crate_still_reports_exactly_what_it_reported_before` | `src\cli\handlers\dead_code_subtree_scope_tests.rs:280:5` | assertion `left == right` failed |
| `cli::handlers::dead_code_handlers::workspace_root_tests::the_verdict_names_the_manifest_that_governs_the_path` | `src\cli\handlers\dead_code_workspace_root_tests.rs:293:5` | the detail must name the manifest it read the target shape from: LibraryTargetReport { verdict: "library", detail: "cargo: \\\\?\\C:\\Users\\<user>… |
| `cli::handlers::enforce_handlers::flag_fidelity_tests::exclude_pattern_removes_the_violation_and_the_score_agrees` | `src\cli\handlers\enforce_handlers\flag_fidelity_tests.rs:170:5` | assertion `left == right` failed: --exclude 'src/*' still reported the violation in src/lib.rs: [QualityViolation { violation_type: "not_measured", se… |
| `cli::handlers::hooks_command_handlers::commit_enforcement_tests::without_a_roadmap_the_hook_keeps_the_ticket_pattern_and_cb2113_skips` | `src\cli\handlers\hooks_command_handlers\commit_enforcement_tests.rs:366:5` | without a roadmap a ticket reference matching the pattern is accepted |
| `cli::handlers::hooks_command_handlers::commit_enforcement_tests::the_hook_and_cb2113_give_the_same_verdict_on_the_same_message` | `src\cli\handlers\hooks_command_handlers\commit_enforcement_tests.rs:345:9` | assertion `left == right` failed: the trailer in the last paragraph names an open item: the hook refuses while CB-2113 accepts — one predicate, two ca… |
| `cli::handlers::hooks_command_handlers::tests::coverage_tests::test_run_all_files_mode` | `src\cli\handlers\hooks_command_handlers\..\hooks_command_handlers_tests.rs:775:9` | assertion failed: result.success |
| `cli::handlers::hooks_command_handlers::tests::coverage_tests::test_run_with_hook_verbose` | `src\cli\handlers\hooks_command_handlers\..\hooks_command_handlers_tests.rs:745:9` | assertion failed: result.success |
| `cli::handlers::provability_handler::coverage_tests::test_resolve_function_targets_with_specific_functions` | `src\cli\handlers\provability_handler_tests.rs:424:55` | called `Result::unwrap()` on an `Err` value: no function named `src/lib.rs:helper` found under C:\Users\<USER~1>\AppData\Local\Temp\.tmpCqHoqj |
| `cli::handlers::qa_mcp_sweep::agent_quality_hook_tests::both_client_configs_point_at_the_same_entrypoint` | `src\cli\handlers\qa_mcp_sweep.rs:1317:9` | assertion `left == right` failed: this repo's .agents/hooks.json no longer matches what `pmat init --target agy` emits |
| `cli::handlers::qa_work_handler::impl_spec_tests::test_resolve_spec_path_gh_prefix` | `src\cli\handlers\qa_work_handler\mod.rs:797:9` | assertion failed: resolved.ends_with("gh-42.md") |
| `cli::handlers::query_handler::indexing::tests::test_backfill_results_source_after_incremental_update` | `src\cli\handlers\query_handler\indexing.rs:508:14` | alpha should be indexed |
| `cli::handlers::refactor_docs_handlers::tests::tests::test_create_backup` | `src\cli\handlers\refactor_docs_tests_async_serial.rs:215:56` | called `Result::unwrap()` on an `Err` value: Failed to backup C:\Users\<USER~1>\AppData\Local\Temp\.tmpVy4hr9\to_backup.txt |
| `cli::handlers::split_handler::missing_target_tests::missing_file_reports_the_os_error` | `src\cli\handlers\split_handler.rs:277:9` | message must carry the OS error, got: Cannot read C:/path/that/does/not/exist.rs: The system cannot find the path specified. (os error 3) |
| `cli::handlers::work_contract_binding::tests::resolve_honors_pmat_contracts_path` | `src\cli\handlers\work_contract_binding.rs:288:9` | assertion `left == right` failed |
| `cli::handlers::work_handlers::roadmap_fragments_wiring_tests::roadmap_fragments_an_id_spent_by_a_fragment_on_another_ref_is_spent` | `src\cli\handlers\work_handlers\..\..\..\tests\roadmap_fragments_wiring_tests.rs:685:5` | assertion `left == right` failed |
| `cli::handlers::work_handlers::work_add_single_authority_tests::work_add_single_authority_counts_ids_on_other_refs` | `src\cli\handlers\work_handlers\..\..\..\tests\work_add_single_authority_tests.rs:213:5` | an id spent on any ref of the repository is spent: the mint must be PMAT-021, ids are ["PMAT-001", "PMAT-002", "PMAT-003", "PMAT-004", "PMAT-005", "PM… |
| `cli::handlers::work_handlers::work_add_single_authority_tests::work_add_single_authority_lock_lives_in_the_git_common_dir` | `src\cli\handlers\work_handlers\..\..\..\tests\work_add_single_authority_tests.rs:399:5` | assertion `left == right` failed: every checkout of a repository must resolve ONE authority file |
| `cli::handlers::work_ledger::work_agent_claims_tests::normalize_makes_absolute_paths_repo_relative_and_refuses_outsiders` | `src\cli\handlers\work_agent_claims_tests.rs:63:73` | ok: claim path 'C:\Users\<USER~1>\AppData\Local\Temp\.tmpJpwCPX\src\lib.rs' is outside the project root \\?\C:\Users\<user>\AppData\Local\Temp\.tmp… |
| `cli::hook_clippy_gate_tests::a_failing_format_gate_prints_why_instead_of_dying_on_the_line` | `src\cli\hook_clippy_gate_tests.rs:383:10` | run hook: Os { code: 3, kind: NotFound, message: "The system cannot find the path specified." } |
| `cli::hook_clippy_gate_tests::both_hook_installers_embed_the_identical_clippy_gate` | `src\cli\hook_clippy_gate_tests.rs:464:5` | assertion `left == right` failed: the two installers' clippy gates have diverged; whichever installer ran last would decide whether clippy is enforced |
| `cli::hook_clippy_gate_tests::the_hook_allows_a_commit_when_the_clippy_stage_is_green` | `src\cli\hook_clippy_gate_tests.rs:294:10` | run hook: Os { code: 3, kind: NotFound, message: "The system cannot find the path specified." } |
| `cli::hook_clippy_gate_tests::the_hook_fails_closed_when_pmat_is_missing_from_a_cargo_project` | `src\cli\hook_clippy_gate_tests.rs:424:10` | run hook: Os { code: 3, kind: NotFound, message: "The system cannot find the path specified." } |
| `cli::hook_clippy_gate_tests::the_hook_refuses_when_the_clippy_stage_comes_back_red` | `src\cli\hook_clippy_gate_tests.rs:294:10` | run hook: Os { code: 3, kind: NotFound, message: "The system cannot find the path specified." } |
| `mcp_pmcp::quality_handlers::tests::test_quality_gate_tool_strict_false` | `src\cli\analysis_utilities\quality_checks_part2_security_duplicates.rs:162:31` | can call blocking only when running on the multi-threaded runtime |
| `mcp_pmcp::quality_handlers::tests::test_quality_gate_tool_with_strict` | `src\cli\analysis_utilities\quality_checks_part2_security_duplicates.rs:162:31` | can call blocking only when running on the multi-threaded runtime |
| `mcp_pmcp::surface_verdict_and_scope_tests::one_finding_is_described_the_same_way_whether_the_file_or_its_directory_is_named` | `src\cli\analysis_utilities\quality_checks_part2_security_duplicates.rs:162:31` | can call blocking only when running on the multi-threaded runtime |
| `mcp_pmcp::tool_functions::gate_parity_tests::a_directory_gate_accounts_for_every_advertised_check` | `src\cli\analysis_utilities\quality_checks_part2_security_duplicates.rs:162:31` | can call blocking only when running on the multi-threaded runtime |
| `mcp_pmcp::tool_functions::gate_parity_tests::the_two_surfaces_report_the_same_findings_for_one_directory` | `src\cli\analysis_utilities\quality_checks_part2_security_duplicates.rs:162:31` | can call blocking only when running on the multi-threaded runtime |
| `mcp_pmcp::tool_functions::mcp_quality_gate_parity_tests::test_unparseable_source_is_not_an_a_grade_pass` | `src\cli\analysis_utilities\quality_checks_part2_security_duplicates.rs:162:31` | can call blocking only when running on the multi-threaded runtime |
| `mcp_pmcp::tool_functions::tests::coverage_tests::test_check_quality_gates_project_verdict_not_inverted` | `src\cli\analysis_utilities\quality_checks_part2_security_duplicates.rs:162:31` | can call blocking only when running on the multi-threaded runtime |
| `mcp_pmcp::tool_functions::tests::coverage_tests::test_mcp_grade_is_the_one_wire_spelling_not_a_variant_name` | `src\cli\analysis_utilities\quality_checks_part2_security_duplicates.rs:162:31` | can call blocking only when running on the multi-threaded runtime |
| `mcp_pmcp::tool_manifest::tests::committed_mcp_json_is_pinned_to_the_renderer` | `src\mcp_pmcp\tool_manifest.rs:634:9` | assertion `left == right` failed: mcp.json is not what the renderer produces — regenerate it: cargo test --lib regenerate_mcp_json -- --ignored |
| `services::agent_context::function_index::tests::test_file_checksums_populated` | `src\services\agent_context\function_index\tests_persistence.rs:471:5` | assertion failed: index.manifest.file_checksums.contains_key("src/lib.rs") |
| `services::agent_context::function_index::tests::test_verify_slice_rehashes_a_file_the_stat_fast_path_would_skip` | `src\services\agent_context\function_index\tests_persistence.rs:783:5` | assertion `left == right` failed: a file in the verification slice must be re-hashed despite agreeing stats |
| `services::agent_context::query::coverage::profdata::tests::test_target_dir_from_cargo_config_absolute_preserved` | `src\services\agent_context\query\coverage\profdata.rs:553:9` | assertion `left == right` failed |
| `services::agent_context::query::coverage::tests::tests::test_try_load_lcov_info` | `src\services\agent_context\query\coverage\tests.rs:566:9` | assertion failed: map.contains_key("src/lib.rs") |
| `services::cargo_dead_code_analyzer::target_isolation_tests::the_key_is_pinned_across_builds` | `src\services\cargo_dead_code_analyzer\target_isolation_tests.rs:78:5` | assertion `left == right` failed |
| `services::dead_code_multi_language::library_tests::a_workspace_manifest_decides_nothing_about_a_crate_it_does_not_declare` | `src\services\dead_code_multi_language_library_tests.rs:426:5` | the reason must name the manifest it read: the nearest Cargo.toml above it, \\?\C:\Users\<user>\AppData\Local\Temp\.tmp2ncLLn\Cargo.toml, is a WORK… |
| `services::dead_code_multi_language::tests::test_lua_test_files_excluded_from_definitions` | `src\services\dead_code_multi_language.rs:437:9` | Test functions excluded |
| `services::facades::defect_prediction_facade::tests::test_include_exclude_accept_globs` | `src\services\facades\defect_prediction_facade.rs:1521:9` | assertion failed: DefectPredictionFacade::matches_filters(&file, &req) |
| `services::metrics_ratchet::drive_tests::every_committed_metric_command_still_measures` | `src\services\metrics_ratchet\drive_tests.rs:319:17` | metric `unwrap_calls_shipped_code` no longer measures: command exceeded the 1800s measurement timeout and was killed: o=$(CLIPPY_CONF_DIR=. CARGO_TARG… |
| `services::metrics_ratchet::drive_tests::no_matches_is_a_zero_not_a_failure` | `src\services\metrics_ratchet\drive_tests.rs:57:5` | assertion `left == right` failed |
| `services::metrics_ratchet::coherence_drive_tests::arm_a_max_unwrap_calls_is_judged_against_a_live_measurement` | `src\services\metrics_ratchet\coherence_drive_tests.rs:128:10` | the unwrap gate must carry a live measurement, not a remembered number |
| `services::metrics_ratchet::drive_tests::zero_is_still_reachable_when_it_is_declared_or_already_the_baseline` | `src\services\metrics_ratchet\drive_tests.rs:542:5` | assertion `left == right` failed |
| `services::metrics_ratchet::drive_tests::the_committed_ratchet_holds_at_head` | `src\services\metrics_ratchet\drive_tests.rs:288:9` | .pmat-ratchet.toml is red at HEAD: |
| `services::metrics_ratchet::tests::a_grandchild_holding_the_pipe_is_left_alone_when_the_shell_exits_first` | `src\services\metrics_ratchet\tests.rs:242:5` | measure() must return promptly (bounded by DRAIN_GRACE), even though `bash` itself exits immediately and a backgrounded grandchild is left holding the… |
| `services::project_meta_detector::tests::test_detect_metadata_files` | `src\services\project_meta_detector.rs:222:9` | assertion `left == right` failed |
| `services::project_meta_detector::tests::test_detect_various_makefile_variants` | `src\services\project_meta_detector.rs:259:9` | assertion `left == right` failed |
| `services::metrics_ratchet::tests::child_process_sees_depth_incremented` | `src\services\metrics_ratchet\tests.rs:56:5` | assertion `left == right` failed: a command run by a depth-0 measurement must see depth 1 |
| `services::metrics_ratchet::tests::depth_env_absent_by_default_measures_normally` | `src\services\metrics_ratchet\tests.rs:45:5` | assertion `left == right` failed |
| `services::repo_score::scorers::hygiene_scorer::coverage_tests_unit::test_cruft_desktop_ini` | `src\services\repo_score\scorers\hygiene_scorer\coverage_tests_unit.rs:183:5` | Should penalize desktop.ini |
| `services::repo_score::scorers::hygiene_scorer::coverage_tests_unit::test_cruft_thumbs_db` | `src\services\repo_score\scorers\hygiene_scorer\coverage_tests_unit.rs:169:5` | Should penalize Thumbs.db |
| `services::repo_score::scorers::hygiene_scorer::coverage_tests_unit::test_team_files_atom` | `src\services\repo_score\scorers\hygiene_scorer\coverage_tests_unit.rs:359:5` | Should penalize .atom directory |
| `services::repo_score::scorers::hygiene_scorer::coverage_tests_unit::test_team_files_fleet` | `src\services\repo_score\scorers\hygiene_scorer\coverage_tests_unit.rs:330:5` | Should penalize .fleet directory |
| `services::repo_score::scorers::hygiene_scorer::coverage_tests_unit::test_team_files_idea` | `src\services\repo_score\scorers\hygiene_scorer\coverage_tests_unit.rs:259:5` | Should penalize .idea directory |
| `services::repo_score::scorers::hygiene_scorer::coverage_tests_unit::test_team_files_settings_dir` | `src\services\repo_score\scorers\hygiene_scorer\coverage_tests_unit.rs:316:5` | Should penalize .settings directory |
| `services::repo_score::scorers::hygiene_scorer::coverage_tests_unit::test_team_files_visual_studio` | `src\services\repo_score\scorers\hygiene_scorer\coverage_tests_unit.rs:273:5` | Should penalize .vs directory |
| `services::repo_score::scorers::hygiene_scorer::coverage_tests_unit::test_team_files_vs_code` | `src\services\repo_score\scorers\hygiene_scorer\coverage_tests_unit.rs:244:5` | Should penalize .vscode directory |
| `services::repo_score::scorers::hygiene_scorer::tests::test_hygiene_scorer_with_team_files` | `src\services\repo_score\scorers\hygiene_scorer\tests.rs:88:5` | assertion failed: result.score >= 12.5 && result.score <= 13.5 |
| `services::rust_project_score::command_runner::tests::test_command_runner_output_structure` | `src\services\rust_project_score\command_runner.rs:332:9` | assertion failed: output.is_some() |
| `services::rust_project_score::command_runner::tests::test_run_with_multiple_args` | `src\services\rust_project_score\command_runner.rs:356:9` | assertion failed: output.is_some() |
| `services::rust_project_score::command_runner::tests::test_run_with_timeout_success` | `src\services\rust_project_score\command_runner.rs:154:9` | assertion failed: output.is_some() |
| `services::rust_project_score::command_runner::tests::test_run_with_timeout_timeout_behavior` | `src\services\rust_project_score\command_runner.rs:311:9` | assertion failed: output.is_some() |
| `services::spec_epic::tests::the_header_goal_mode_documents_parses_with_its_inline_comments` | `src\services\spec_epic\tests.rs:367:10` | §4.3 documents the header in a yaml fence |
| `services::tdg_baseline::tests::the_committed_baseline_is_the_measured_count` | `src\services\tdg_baseline.rs:799:9` | CB-200 REGRESSED: 1704 definitions below grade A, recorded baseline 1680. 24 new one(s) since the baseline was taken. Fix them, or revert what added t… |
| `services::test_env_hygiene::tests::every_pmat_spawn_is_hygienic_or_ledgered` | `src\services\test_env_hygiene.rs:377:9` | tests that spawn the pmat binary must not inherit the ambient environment — several variables change what the binary DOES, so the assertions compare a… |
| `services::ttg::score::tests::declarations_are_charged_for_size_on_the_same_curve` | `src\services\ttg\score.rs:655:13` | assertion `left == right` failed: T = 5000 |
| `services::unrun_tests::tests::include_splices_into_the_current_module_without_growing_the_path` | `src\services\unrun_tests\tests.rs:248:5` | assertion `left == right` failed: the body's file, not the includer's |
| `services::unrun_tests::tests::the_committed_ledger_matches_the_tree` | `src\services\unrun_tests\tests.rs:581:5` | the unrun-tests ledger has drifted. |
| `tdg::storage_backend::extended_tests::test_libsql_backend_file_not_found_size` | `src\tdg\storage_backend_tests_extended_part2.rs:305:14` | a deleted database file must not be measured as 0 bytes: 4096 |
| `tdg::storage_backend::tests::size_on_disk_fails_loudly_when_the_db_file_is_gone` | `src\tdg\storage_backend.rs:283:40` | called `Result::unwrap()` on an `Err` value: Os { code: 32, kind: Uncategorized, message: "The process cannot access the file because it is being used… |
| `test_performance::throughput_verdict_tests::test_memory_sample_reads_kb_resolution` | `src\test_performance_suite.rs:423:43` | linux test host exposes /proc/self/status: cannot measure memory: resident-set reporting is Linux-only on this build |
| `utils::pmat_cache_dir::tests_sweep::only_the_idle_entries_are_reclaimed` | `src\utils\pmat_cache_dir.rs:817:14` | age the entry: Os { code: 5, kind: PermissionDenied, message: "Access is denied." } |
