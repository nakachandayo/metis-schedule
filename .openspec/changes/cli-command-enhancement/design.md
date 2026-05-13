# Design: CLI Command Configuration Enhancement

## Architecture

### CLI Command Resolution

```
Task.cli_command (Optional<CliCommand>)
         │
         ▼ null
┌────────────────────┐
│   Use config.json  │
│   default values  │
└────────────────────┘
         │
         ▼ non-null
┌────────────────────┐
│ Use task-specific  │
│ cli_command values │
└────────────────────┘
```

### Command Building

```
Input:
  task.prompt = "检索主题A"
  cli_command = CliCommand {
    exe: "claude",
    args: [
      CliArg { name: "-p", value: "" },
      CliArg { name: "--permission-mode", value: "acceptEdits" }
    ]
  }

Processing:
  1. exe = "claude"
  2. For each CliArg:
     - name = "-p" → add "-p" to args
     - name = "--permission-mode", value = "acceptEdits" → add "--permission-mode" and "acceptEdits"
  3. Add task.prompt as last argument

Output:
  Command::new("claude")
    .args(["-p", "--permission-mode", "acceptEdits", "检索主题A"])
    .output()
```

## Data Structures

### Rust - CliArg

```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct CliArg {
    pub name: String,   // "-p", "--permission-mode"
    pub value: String,  // 可为空 ""
}
```

### Rust - CliCommand

```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct CliCommand {
    pub exe: String,           // "claude"
    pub args: Vec<CliArg>,     // 参数列表
}
```

### Rust - Task

```rust
pub struct Task {
    pub id: String,
    pub task_type: String,
    pub time: String,
    pub days_of_week: Option<Vec<u8>>,
    pub once_date: Option<String>,
    pub label: Option<String>,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cli_command: Option<CliCommand>,  // null = 使用全局默认
    #[serde(skip)]
    pub next_trigger: Option<DateTime<Local>>,
}
```

### Rust - Config

```rust
#[derive(Serialize, Deserialize)]
pub struct Config {
    pub command: String,      // "claude"
    pub args: Vec<CliArg>,    // 默认参数列表
}

impl Default for Config {
    fn default() -> Self {
        Self {
            command: "claude".to_string(),
            args: vec![
                CliArg { name: "-p".to_string(), value: "".to_string() },
                CliArg { name: "--permission-mode".to_string(), value: "acceptEdits".to_string() },
            ],
        }
    }
}
```

### JSON - config.json

```json
{
  "command": "claude",
  "args": [
    { "name": "-p", "value": "" },
    { "name": "--permission-mode", "value": "acceptEdits" }
  ]
}
```

### JSON - tasks.json

```json
[
  {
    "id": "xxx",
    "task_type": "daily",
    "time": "08:00",
    "prompt": "检索主题A",
    "cli_command": null,
    ...
  },
  {
    "id": "yyy",
    "task_type": "once",
    "time": "14:00",
    "prompt": "检索主题B",
    "cli_command": {
      "exe": "claude",
      "args": [
        { "name": "-p", "value": "" },
        { "name": "--model", "value": "sonnet" }
      ]
    },
    ...
  }
]
```

## Frontend UI

### Collapsed State (Default)

```
┌─────────────────────────────────────────┐
│ 检索主题 *：                             │
│ [检索主题内容...________________]        │
│                                         │
│ ▶ CLI 命令配置                           │
│   ☑ 使用全局默认                        │
│     claude -p --permission-mode acceptEdits │
└─────────────────────────────────────────┘
```

### Expanded State

```
┌─────────────────────────────────────────┐
│ 检索主题 *：                             │
│ [检索主题内容...________________]        │
│                                         │
│ ▼ CLI 命令配置                           │
│   ☐ 使用全局默认                        │
│                                         │
│   命令: [claude                   ]     │
│                                         │
│   Args:                                 │
│     [ -p ] [ value: ________ ] [✕]     │
│     [ --permission-mode ] [ acceptEdits ] [✕] │
│     [ + 添加参数 ]                       │
│                                         │
│   预览: claude -p --permission-mode acceptEdits 检索主题内容 │
└─────────────────────────────────────────┘
```

### Interactions

| Action | Behavior |
|--------|----------|
| Click "▶ CLI 命令配置" | Expand to show customization fields |
| Check "使用全局默认" | Disable fields, show config preview |
| Uncheck "使用全局默认" | Enable fields for customization |
| Click "+ 添加参数" | Add new row: [input] [input] [✕] |
| Click "✕" on arg row | Remove that argument row |
| Type in name field | Update preview command |
| Type in value field | Update preview command |

## Scheduler Logic

### build_cli_command()

```rust
fn build_cli_command(task: &Task, config: &Config) -> (String, Vec<String>) {
    let cli = task.cli_command.as_ref().unwrap_or(config);

    let exe = cli.command.clone();
    let mut args: Vec<String> = Vec::new();

    for arg in &cli.args {
        args.push(arg.name.clone());
        if !arg.value.is_empty() {
            args.push(arg.value.clone());
        }
    }

    args.push(task.prompt.clone());

    (exe, args)
}
```

### spawn_cli()

```rust
fn spawn_cli(config: &Config, task: &Task) -> Result<CliResult, String> {
    let exe_dir = exe_dir()?;
    let log_path = exe_dir.join("cli_execution_log.json");

    let (exe, args) = build_cli_command(task, config);

    let output = Command::new(&exe)
        .args(&args)
        .current_dir(&exe_dir)
        .output()
        .map_err(|e| format!("Failed to execute {}: {}", exe, e))?;

    let result = CliResult {
        executed_at: Local::now().to_rfc3339(),
        command: format!("{} {}", exe, args.join(" ")),
        prompt: task.prompt.clone(),
        success: output.status.success(),
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    };

    append_to_log(&log_path, &result, 10)?;

    if !result.success {
        return Err(format!("CLI failed (exit {:?}): {}", result.exit_code, result.stderr));
    }

    Ok(result)
}
```

## Execution Log

### File: cli_execution_log.json

```json
[
  {
    "executed_at": "2026-05-08T13:30:00+08:00",
    "command": "claude -p --permission-mode acceptEdits 检索主题A",
    "prompt": "检索主题A",
    "success": true,
    "exit_code": 0,
    "stdout": "...",
    "stderr": ""
  }
]
```

## File Changes

| File | Changes |
|------|---------|
| `src-tauri/src/config.rs` | New `CliArg`, `Config` with `command` + `args` |
| `src-tauri/src/task.rs` | New `CliCommand`, `CliArg`, Task with optional `cli_command` |
| `src-tauri/src/task_manager.rs` | Update load/save for new config/task format |
| `src-tauri/src/scheduler.rs` | `build_cli_command()`, `spawn_cli()` update |
| `src/types.ts` | Add `CliCommand`, `CliArg` types |
| `src/App.vue` | CLI config UI: collapsible, add/remove args |
| `config.json` | Update format to `{ command, args }` |
| `tasks.json` | Migration: add `cli_command: null` to existing tasks |

## Migration Strategy

### tasks.json Migration

```rust
// On load, if task.cli_command is missing (old format), set to null
fn migrate_task(task: &mut Task) {
    if task.cli_command.is_none() {
        task.cli_command = None;  // Use global default
    }
}
```

### config.json Migration

Old format: `{ "command": "claude -p" }`
New format: `{ "command": "claude", "args": [...] }`

Migration on load:
```rust
if config.args.is_empty() {
    // Old format detected, migrate
    let parts: Vec<&str> = config.command.split_whitespace().collect();
    config.command = parts[0].to_string();
    config.args = parts[1..]
        .chunks(2)
        .map(|chunk| CliArg {
            name: chunk[0].to_string(),
            value: chunk.get(1).unwrap_or(&"").to_string(),
        })
        .collect();
}
```
