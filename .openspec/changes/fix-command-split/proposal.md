# Fix: Command Splitting and CLI Execution Logging

## 问题描述

CLI 命令无法正确执行。`config.json` 中配置的 `command: "claude -p"` 被作为单一可执行文件名传递给 `Command::new()`，导致系统尝试寻找名为 `claude -p`（带空格）的文件而非 `claude` 程序。

### 根因

```rust
// 原代码 (scheduler.rs)
Command::new(command)  // "claude -p" 被当作单一文件名
    .arg(prompt)       // prompt 作为参数传递
    .spawn()
```

### 期望行为

```rust
// 正确做法
Command::new("claude")  // 可执行文件名
    .args(["-p", prompt])  // 参数正确拆分
    .output()
```

## 解决方案

### 1. 命令拆分 (`spawn_cli` 函数重写)

将 `config.command` 按空白拆分：
- 第一个元素作为可执行文件
- 剩余元素 + prompt 作为命令行参数

```rust
let parts: Vec<&str> = config_cmd.split_whitespace().collect();
// "claude -p" → ["claude", "-p"]
let exe = parts[0];           // "claude"
let base_args = &parts[1..];  // ["-p"]
let mut full_args = base_args.to_vec();
full_args.push(prompt);       // ["-p", "检索主题"]
```

### 2. 执行结果捕获

改用 `Command::output()` 替代 `spawn()`：
- 等待进程执行完成
- 捕获 stdout、stderr
- 获取退出码

### 3. JSON 执行日志

新增 `cli_execution_log.json` 文件：
- 位置：`exe_dir/cli_execution_log.json`
- 内容：JSON 数组，最新 10 条记录
- 字段：`executed_at`, `command`, `prompt`, `success`, `exit_code`, `stdout`, `stderr`

```json
[
  {
    "executed_at": "2026-05-08T13:30:00+08:00",
    "command": "claude -p",
    "prompt": "检索主题内容",
    "success": true,
    "exit_code": 0,
    "stdout": "...",
    "stderr": ""
  }
]
```

## 改动文件

| 文件 | 改动 |
|------|------|
| `src-tauri/src/scheduler.rs` | `spawn_cli()` 完全重写，新增 `CliResult` 结构体、`append_to_log()` 函数 |

## 行为变化

| 方面 | 修改前 | 修改后 |
|------|--------|--------|
| 命令执行 | `spawn()` 不等待 | `output()` 等待并捕获结果 |
| 成功判断 | 仅 spawn 成功 | `status.success()` 检查退出码 |
| 执行日志 | 无 | JSON 文件保留最新 10 条 |
| 错误处理 | 打印到 stderr | 返回 `Err` + 日志记录 |

## 测试验证

1. 创建一次性任务，prompt 设为 `test_env.bat`
2. 等待执行后检查 `cli_execution_log.json`
3. 验证日志中 `command`、`prompt`、`success`、`exit_code` 字段正确

## 状态

- [x] 已实施
- [ ] 已测试
- [ ] 已归档
