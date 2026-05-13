# Tasks: CLI Command Configuration Enhancement

## Implementation Phases

### Phase 1: Core Data Structures ✅

- [x] 1.1 Create `CliArg` struct in `task.rs`
  - Fields: `name: String`, `value: String`
  - Add Serialize, Deserialize, Clone derives

- [x] 1.2 Create `CliCommand` struct in `task.rs`
  - Fields: `exe: String`, `args: Vec<CliArg>`
  - Add Serialize, Deserialize, Clone derives

- [x] 1.3 Add `cli_command: Option<CliCommand>` to `Task` struct
  - Add `#[serde(skip_serializing_if = "Option::is_none")]`

- [x] 1.4 Update `TaskWithNextTrigger` to include `cli_command`

- [x] 1.5 Update `Config` struct in `config.rs`
  - Change from `{ command: String }` to `{ command: String, args: Vec<CliArg> }`

- [x] 1.6 Update `Config::default()` to set default args

### Phase 2: Task Manager Updates ✅

- [x] 2.1 Update `load_config()` to handle new config format
  - Include migration logic from old format

- [x] 2.2 Update `save_config()` to serialize new format

- [x] 2.3 Update `add()` to accept new Task format (cli_command optional)

- [x] 2.4 Update `update()` to handle cli_command changes

- [x] 2.5 Ensure `load()` migrates old tasks.json format

### Phase 3: Scheduler Updates ✅

- [x] 3.1 Create `build_cli_command(config, task)` function
  - Return (exe, args) tuple

- [x] 3.2 Update `spawn_cli()` to use `build_cli_command()`

- [x] 3.3 Ensure `get_tasks_needing_trigger()` passes config reference

- [x] 3.4 Verify `append_to_log()` still works correctly

### Phase 4: Frontend Types ✅

- [x] 4.1 Add `CliArg` interface in `types.ts`

- [x] 4.2 Add `CliCommand` interface in `types.ts`

- [x] 4.3 Add `cli_command?: CliCommand | null` to `Task` interface

### Phase 5: Frontend UI ✅

- [x] 5.1 Create collapsible CLI config section in TaskEditDialog
  - Default collapsed state
  - Toggle on click

- [x] 5.2 Add "使用全局默认" checkbox
  - When checked: show preview only, disable fields
  - When unchecked: enable customization fields

- [x] 5.3 Add command input field
  - Bound to `cliCommand.exe`

- [x] 5.4 Create dynamic args list UI
  - Each row: [name input] [value input] [delete button]
  - "添加参数" button to add new row

- [x] 5.5 Add command preview text
  - Shows full command: `exe args... prompt`

- [x] 5.6 Update save logic to handle null cli_command for global default

- [x] 5.7 Ensure TaskWithNextTrigger displays correctly

### Phase 6: Configuration Files ✅

- [x] 6.1 Update `config.json` format to new structure
  - Old config deleted, new will be created with defaults

- [x] 6.2 Verify `tasks.json` migration on first run

### Phase 7: Testing ⏳

- [ ] 7.1 Manual test: Create task with default CLI command
  - Verify execution log shows correct command

- [ ] 7.2 Manual test: Create task with custom CLI command
  - Verify task uses custom instead of default

- [ ] 7.3 Manual test: Add/remove args
  - Verify preview updates correctly

- [ ] 7.4 Manual test: Verify cli_execution_log.json contains correct entries

- [ ] 7.5 Manual test: Once task executes and is removed from list

### Phase 8: Documentation ⏳

- [x] 8.1 Update proposal.md status to Approved

- [x] 8.2 Update design.md with any changes during implementation

- [ ] 8.3 Archive change after testing complete

## Dependencies

- Phase 1 must complete before Phase 2 ✅
- Phase 2 must complete before Phase 3 ✅
- Phase 4 must complete before Phase 5 ✅
- Phase 6 depends on Phase 1-5 ✅
- Phase 7 depends on Phase 1-6 ✅

## Notes

- Keep `tasks-changed` event emission for frontend refresh
- Keep `cli_execution_log.json` with 10-entry limit
- Ensure backward compatibility with old tasks.json format
- config.json does not need backward compatibility (auto-regenerate)

## Implementation Summary

### Completed Changes

| File | Changes |
|------|---------|
| `src-tauri/src/task.rs` | Added `CliArg`, `CliCommand` structs; Added `cli_command` to `Task` and `TaskWithNextTrigger` |
| `src-tauri/src/config.rs` | Changed to `command` + `args` format |
| `src-tauri/src/task_manager.rs` | Updated load/save for new format; Added migration from old format |
| `src-tauri/src/scheduler.rs` | Rewrote `build_cli_command()` and `spawn_cli()` to use new structure |
| `src/types.ts` | Added `CliArg`, `CliCommand` interfaces |
| `src/App.vue` | Added collapsible CLI config UI with add/remove args |

### Default CLI Command

```
claude -p --permission-mode acceptEdits <prompt>
```

| Element | Source |
|---------|--------|
| `claude` | config.command |
| `-p` | config.args[0] |
| `--permission-mode` | config.args[1].name |
| `acceptEdits` | config.args[1].value |
| `<prompt>` | task.prompt |
