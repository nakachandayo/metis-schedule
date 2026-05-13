mod config;
mod scheduler;
mod task;
mod task_manager;

use config::Config;
use std::sync::Arc;
use task::Task;
use task_manager::TaskManager;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

pub struct AppState {
    pub task_manager: Arc<TaskManager>,
}

#[tauri::command]
fn get_tasks(state: tauri::State<AppState>) -> Vec<task::TaskWithNextTrigger> {
    state.task_manager.get_all()
}

#[tauri::command]
fn add_task(task: Task, state: tauri::State<AppState>) -> Result<(), String> {
    state.task_manager.add(task)
}

#[tauri::command]
fn update_task(task: Task, state: tauri::State<AppState>) -> Result<(), String> {
    state.task_manager.update(task)
}

#[tauri::command]
fn delete_task(id: String, state: tauri::State<AppState>) -> Result<(), String> {
    state.task_manager.delete(&id)
}

#[tauri::command]
fn get_tasks_for_date(
    date: String,
    state: tauri::State<AppState>,
) -> Result<Vec<task::TaskWithNextTrigger>, String> {
    let naive_date =
        chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d").map_err(|e| format!("{}", e))?;
    Ok(state.task_manager.get_tasks_for_date(naive_date))
}

#[tauri::command]
fn get_config(state: tauri::State<AppState>) -> Result<Config, String> {
    state.task_manager.load_config()
}

#[tauri::command]
fn save_config(config: Config, state: tauri::State<AppState>) -> Result<(), String> {
    state.task_manager.save_config(&config)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let task_manager = Arc::new(TaskManager::new());
    if let Err(e) = task_manager.load() {
        eprintln!("Failed to load tasks on startup: {}", e);
    }
    if let Err(e) = task_manager.load_config() {
        eprintln!("Failed to load config on startup: {}", e);
    }

    let app_state = AppState {
        task_manager: task_manager.clone(),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .setup(move |app| {
            let show_item = MenuItem::with_id(app, "show", "打开窗口", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            let app_handle = app.handle().clone();
            let scheduler =
                scheduler::Scheduler::new(task_manager.clone()).with_app_handle(app_handle);

            tauri::async_runtime::spawn(async move {
                scheduler.start().await;
            });

            let window = app.get_webview_window("main").unwrap();
            let window_clone = window.clone();

            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    window_clone.hide().unwrap();
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_tasks,
            add_task,
            update_task,
            delete_task,
            get_tasks_for_date,
            get_config,
            save_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
