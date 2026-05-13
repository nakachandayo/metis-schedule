# Design: CLI Execution Fix

## 目标

修复 CLI 命令执行逻辑，确保 `config.command` 中的多部分命令（如 `claude -p`）能被正确解析和执行。

## 架构

### 命令拆分逻辑

```
config.json: { "command": "claude -p" }
                    ↓
            split_whitespace()
                    ↓
            ["claude", "-p"]
                    ↓
┌─────────────────────────────────────┐
│  exe = "claude"                     │
│  args = ["-p", "<prompt>"]          │
└─────────────────────────────────────┘
                    ↓
            Command::new(exe)
                .args(args)
                .output()
```

### 日志模块

```
append_to_log(result, max=10)
         ↓
    读取现有日志（如果存在）
         ↓
    追加新结果
         ↓
    保留最新10条
         ↓
    写回文件
```

## 数据结构

### CliResult

```rust
struct CliResult {
    executed_at: String,    // RFC3339 时间戳
    command: String,        // 原始命令（来自 config）
    prompt: String,         // 任务 prompt
    success: bool,         // exit_code == 0
    exit_code: Option<i32>, // 进程退出码
    stdout: String,         // 标准输出
    stderr: String,         // 标准错误
}
```

## 错误处理

| 错误类型 | 处理方式 |
|----------|----------|
| 命令为空 | 返回 `Err("Empty command")` |
| 可执行文件不存在 | `Command::output()` 返回 `Err` |
| CLI 业务失败 (exit_code != 0) | 记录日志 + 返回 `Err` |

## 安全性

- 日志文件限制为 10 条，防止无限增长
- 日志文件与 exe 同目录
- stderr/stdout 长度无限制，但 JSON 序列化会处理
