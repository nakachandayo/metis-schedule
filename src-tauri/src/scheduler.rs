use crate::config::Config;
use crate::task_manager::TaskManager;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

#[derive(Serialize, Deserialize, Clone)]
struct CliResult {
    executed_at: String,
    command: String,
    prompt: String,
    success: bool,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

pub struct Scheduler {
    task_manager: Arc<TaskManager>,
    running: Arc<Mutex<bool>>,
    app_handle: Option<tauri::AppHandle>,
}

impl Scheduler {
    pub fn new(task_manager: Arc<TaskManager>) -> Self {
        Self {
            task_manager,
            running: Arc::new(Mutex::new(false)),
            app_handle: None,
        }
    }

    pub fn with_app_handle(mut self, app_handle: tauri::AppHandle) -> Self {
        self.app_handle = Some(app_handle);
        self
    }

    pub async fn start(&self) {
        let mut is_running = self.running.lock().await;
        if *is_running {
            return;
        }
        *is_running = true;
        drop(is_running);

        let task_manager = self.task_manager.clone();
        let running = self.running.clone();
        let app_handle = self.app_handle.clone();

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(30));
            let task_manager_clone = task_manager.clone();

            loop {
                ticker.tick().await;

                let is_running = running.lock().await;
                if !*is_running {
                    break;
                }
                drop(is_running);

                if let Err(e) = task_manager.load() {
                    eprintln!("Scheduler: failed to load tasks: {}", e);
                    continue;
                }

                let tasks_to_trigger = task_manager.get_tasks_needing_trigger();

                for (task, config) in tasks_to_trigger {
                    let task_id = task.id.clone();
                    let task_clone = task.clone();
                    let config_clone = config.clone();
                    let tm_for_cli = task_manager_clone.clone();

                    std::thread::spawn(move || {
                        if let Err(e) = spawn_cli(&tm_for_cli, &config_clone, &task_clone) {
                            eprintln!("Scheduler: CLI error: {}", e);
                        }
                    });

                    if let Err(e) = task_manager.mark_as_triggered(&task_id) {
                        eprintln!("Scheduler: failed to mark task as triggered: {}", e);
                    }

                    if let Some(handle) = &app_handle {
                        if let Err(e) = handle.emit("tasks-changed", ()) {
                            eprintln!("Scheduler: failed to emit event: {}", e);
                        }
                    }
                }
            }
        });
    }

    #[allow(dead_code)]
    pub async fn stop(&self) {
        let mut is_running = self.running.lock().await;
        *is_running = false;
    }
}

fn build_cli_command(config: &Config, task: &crate::task::Task) -> (String, Vec<String>) {
    let exe: String;
    let args_list: &Vec<crate::task::CliArg>;

    if let Some(ref cli_cmd) = task.cli_command {
        exe = cli_cmd.exe.clone();
        args_list = &cli_cmd.args;
    } else {
        exe = config.command.clone();
        args_list = &config.args;
    };

    let mut args: Vec<String> = Vec::new();

    for arg in args_list {
        args.push(arg.name.clone());
        if !arg.value.is_empty() {
            args.push(arg.value.clone());
        }
    }

    args.push(task.prompt.clone());

    (exe, args)
}

fn spawn_cli(
    task_manager: &TaskManager,
    config: &Config,
    task: &crate::task::Task,
) -> Result<CliResult, String> {
    let exe_dir = task_manager.get_exe_dir();
    let log_path = exe_dir.join("cli_execution_log.json");

    let (exe, args) = build_cli_command(config, task);
    let command_str = format!("{} {}", exe, args.join(" "));

    let output = Command::new(&exe)
        .args(&args)
        .current_dir(&exe_dir)
        .output()
        .map_err(|e| format!("Failed to execute {}: {}", exe, e))?;

    let result = CliResult {
        executed_at: Local::now().to_rfc3339(),
        command: command_str,
        prompt: task.prompt.clone(),
        success: output.status.success(),
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    };

    append_to_log(&log_path, &result, 10)?;

    if !result.success {
        return Err(format!(
            "CLI failed (exit {:?}): {}",
            result.exit_code, result.stderr
        ));
    }

    Ok(result)
}

fn append_to_log(path: &Path, result: &CliResult, max_entries: usize) -> Result<(), String> {
    let mut entries: Vec<CliResult> = if path.exists() {
        let content = fs::read_to_string(path).map_err(|e| format!("Failed to read log: {}", e))?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    entries.push(result.clone());

    if entries.len() > max_entries {
        entries = entries.split_off(entries.len() - max_entries);
    }

    let json = serde_json::to_string_pretty(&entries)
        .map_err(|e| format!("Failed to serialize log: {}", e))?;
    fs::write(path, json).map_err(|e| format!("Failed to write log: {}", e))?;

    Ok(())
}
