# MetisSchedule

文献情报定时调度工具 - 基于日历的 CLI 任务调度桌面应用

## 功能特性

- **日历视图** - 月度日历展示，任务类型颜色区分（每日🟢 每周🟠 一次性🔵）
- **后台运行** - 关闭主窗口时最小化到系统托盘，程序继续运行
- **托盘菜单** - 右键"打开窗口"或"退出"
- **CLI 可配置** - 全局默认 CLI 命令 + 单任务覆盖
- **单实例运行** - 重复启动时聚焦已有窗口
- **任务持久化** - 所有数据存储在 exe 同目录的 JSON 文件中

## 快速开始

### 下载预构建版本

前往 [Releases](https://github.com/anomalyco/MetisSchedule/releases) 下载最新版本的 Windows 安装包或独立 exe 文件。

### 运行

1. 解压下载的文件
2. 双击 `MetisSchedule.exe` 运行
3. 程序会在同目录生成 `config.json`、`tasks.json` 等数据文件

## 使用指南

### 创建任务

1. 点击右侧面板的"添加任务"按钮
2. 选择任务类型（每日/每周/一次性）
3. 设置触发时间（如 08:00）
4. 输入检索主题（prompt）
5. 可选：配置自定义 CLI 命令（默认使用全局配置）
6. 点击"确定"保存

### CLI 配置

程序默认使用 `claude -p --permission-mode acceptEdits <prompt>` 执行任务。

可通过任务编辑对话框中的"CLI 命令配置"区域：
- 勾选"使用全局默认"使用 config.json 中的配置
- 取消勾选可设置任务专用的 CLI 命令

### 托盘使用

- 关闭主窗口：程序最小化到托盘，继续后台运行
- 右键托盘图标：选择"打开窗口"或"退出"
- 重复启动程序：自动聚焦已有窗口

## 配置文件

### config.json

CLI 命令全局配置（位于 exe 同目录）：

```json
{
  "command": "claude",
  "args": [
    { "name": "-p", "value": "" },
    { "name": "--permission-mode", "value": "acceptEdits" }
  ]
}
```

### tasks.json

任务列表（位于 exe 同目录）：

```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "task_type": "daily",
    "time": "08:00",
    "days_of_week": null,
    "once_date": null,
    "label": "每日文献调研",
    "prompt": "调研近一年集成电路领域的研究进展",
    "cli_command": null,
    "next_trigger": "2024-01-02T08:00:00"
  }
]
```

## 开发指南

### 环境要求

- Node.js 18+
- Rust 1.70+
- Windows 操作系统

### 安装依赖

```powershell
npm install
```

### 开发模式

```powershell
npm run tauri dev
```

### 构建生产版本

```powershell
npm run tauri build
```

构建产物位于 `src-tauri/target/release/` 目录。

## 项目结构

```
MetisSchedule/
├── src/                    # Vue 前端源码
│   ├── App.vue           # 主组件
│   ├── main.ts          # 入口 + Toast 系统
│   └── types.ts         # TypeScript 类型
├── src-tauri/            # Rust 后端源码
│   ├── src/
│   │   ├── main.rs      # 入口
│   │   ├── lib.rs       # Tauri 设置 + 命令
│   │   ├── task.rs      # 任务数据模型
│   │   ├── task_manager.rs  # 任务 CRUD
│   │   ├── scheduler.rs    # 定时器
│   │   └── config.rs    # CLI 配置
│   └── icons/           # 应用图标
├── config.json           # CLI 配置（运行时生成）
├── tasks.json           # 任务列表（运行时生成）
└── LICENSE             # MIT License
```

## 技术栈

| 层级 | 技术 |
|------|------|
| 前端框架 | Vue 3 + TypeScript |
| 构建工具 | Vite + Tauri |
| 后端 | Rust |
| 异步运行时 | tokio |

## License

[MIT License](LICENSE)