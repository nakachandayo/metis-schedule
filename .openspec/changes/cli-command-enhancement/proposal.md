# Proposal: CLI Command Configuration Enhancement

## Context

LitSchedule 需要支持更灵活的 CLI 命令配置。原始问题：

1. **命令拆分错误**：`config.command = "claude -p"` 被作为单一文件名传递
2. **缺乏任务级覆盖**：所有任务共用同一命令
3. **不支持多参数**：无法传递复杂命令如 `claude -p --permission-mode acceptEdits`

## Requirements

### CLI 命令配置

| 需求 | 描述 |
|------|------|
| 支持多参数 | CLI 命令可包含多个 args |
| 参数值可为空 | args 格式为 `args <value>`，value 可为空 |
| 任务级覆盖 | 不同任务可用不同命令/参数 |
| 全局默认值 | config.json 存储默认命令配置 |
| 折叠默认 | CLI 命令配置在 UI 中默认折叠 |
| 手动增删 | 用户可手动添加/删除参数 |

### 默认命令格式

```
claude -p --permission-mode acceptEdits <prompt>
```

| 元素 | 类型 | 说明 |
|------|------|------|
| `claude` | exe | 可执行文件 |
| `-p` | flag | 无 value（prompt 紧随其后） |
| `--permission-mode` | arg | 有 value `acceptEdits` |
| `acceptEdits` | value | --permission-mode 的值 |
| `<prompt>` | 独立参数 | task.prompt 作为最后一个参数 |

### 执行日志

| 需求 | 描述 |
|------|------|
| JSON 格式 | `cli_execution_log.json` |
| 保留条数 | 最新 10 条 |
| 字段 | executed_at, command, prompt, success, exit_code, stdout, stderr |

### 事件通知

| 需求 | 描述 |
|------|------|
| 前端刷新 | 任务变更（触发/删除）后通知前端刷新 |
| 机制 | Tauri Event `tasks-changed` |

## Design Summary

### 数据结构

**config.json**:
```json
{
  "command": "claude",
  "args": [
    { "name": "-p", "value": "" },
    { "name": "--permission-mode", "value": "acceptEdits" }
  ]
}
```

**Task**:
```rust
struct Task {
    id: String,
    task_type: String,
    time: String,
    days_of_week: Option<Vec<u8>>,
    once_date: Option<String>,
    label: Option<String>,
    prompt: String,
    cli_command: Option<CliCommand>,  // null = 使用全局默认
    next_trigger: Option<DateTime<Local>>,
}

struct CliCommand {
    exe: String,                              // 可执行文件
    args: Vec<CliArg>,                         // 参数列表
}

struct CliArg {
    name: String,      // 参数名，如 "-p", "--permission-mode"
    value: String,     // 参数值，可为空
}
```

### 前端 UI

- CLI 命令配置**默认折叠**
- 勾选「使用全局默认」时，隐藏自定义输入
- 展开后显示命令和参数编辑区
- Args 可手动添加/删除
- 底部预览完整命令

### 执行流程

```
scheduler.trigger()
    ↓
build_cli_command(task, config)
    ↓
exe = task.cli_command.exe || config.command
args = task.cli_command.args || config.args
full_args = [...exe_args, ...config_args, task.prompt]
    ↓
Command::new(exe).args(full_args).output()
    ↓
append_to_log(result, 10)
    ↓
emit("tasks-changed")
```

## Status

- [x] Draft
- [x] Approved
- [x] Implemented
- [ ] Tested
- [ ] Archived

## Implementation

| Phase | Status |
|-------|--------|
| Phase 1: Core Data Structures | ✅ Complete |
| Phase 2: Task Manager Updates | ✅ Complete |
| Phase 3: Scheduler Updates | ✅ Complete |
| Phase 4: Frontend Types | ✅ Complete |
| Phase 5: Frontend UI | ✅ Complete |
| Phase 6: Configuration Files | ✅ Complete |
| Phase 7: Testing | ⏳ Pending |
| Phase 8: Documentation | ⏳ Pending |

## Changes

| File | Changes |
|------|---------|
| `task.rs` | Added `CliArg`, `CliCommand` structs; Added `cli_command` to `Task` |
| `config.rs` | Changed to `command` + `args` format with defaults |
| `task_manager.rs` | Updated load/save; Added migration from old format |
| `scheduler.rs` | Rewrote `build_cli_command()` and `spawn_cli()` |
| `types.ts` | Added `CliArg`, `CliCommand` interfaces |
| `App.vue` | Added collapsible CLI config UI |
