use crate::config::Config;
use crate::task::{CliArg, Task, TaskWithNextTrigger};
use chrono::{DateTime, Datelike, Local, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct TaskManager {
    tasks: Mutex<Vec<Task>>,
    config_cache: Mutex<Option<Config>>,
    exe_dir_cache: Mutex<Option<PathBuf>>,
}

impl TaskManager {
    pub fn new() -> Self {
        let exe_dir = Self::compute_exe_dir();
        Self {
            tasks: Mutex::new(Vec::new()),
            config_cache: Mutex::new(None),
            exe_dir_cache: Mutex::new(Some(exe_dir)),
        }
    }

    fn compute_exe_dir() -> PathBuf {
        std::env::current_exe()
            .map(|p| p.parent().map(|p| p.to_path_buf()).unwrap_or_default())
            .unwrap_or_default()
    }

    pub fn get_exe_dir(&self) -> PathBuf {
        self.exe_dir_cache
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(Self::compute_exe_dir)
    }

    pub fn get_config(&self) -> Config {
        let mut cache = self.config_cache.lock().unwrap();
        if let Some(ref config) = *cache {
            return config.clone();
        }
        let config = self.load_config().unwrap_or_else(|_| Config::default());
        *cache = Some(config.clone());
        config
    }

    pub fn load(&self) -> Result<(), String> {
        let tasks_path = self.tasks_path();
        if tasks_path.exists() {
            let content = fs::read_to_string(&tasks_path)
                .map_err(|e| format!("Failed to read tasks.json: {}", e))?;
            let loaded: Vec<Task> = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse tasks.json: {}", e))?;
            let mut tasks = self.tasks.lock().unwrap();
            *tasks = loaded;
        } else {
            fs::write(&tasks_path, "[]")
                .map_err(|e| format!("Failed to create tasks.json: {}", e))?;
        }
        self.recalculate_all_next_triggers();
        Ok(())
    }

    pub fn save(&self) -> Result<(), String> {
        let tasks_path = self.tasks_path();
        let tasks = self.tasks.lock().unwrap();
        let content = serde_json::to_string_pretty(&*tasks)
            .map_err(|e| format!("Failed to serialize tasks: {}", e))?;
        fs::write(&tasks_path, content)
            .map_err(|e| format!("Failed to write tasks.json: {}", e))?;
        Ok(())
    }

    pub fn get_all(&self) -> Vec<TaskWithNextTrigger> {
        let tasks = self.tasks.lock().unwrap();
        tasks.iter().map(TaskWithNextTrigger::from).collect()
    }

    pub fn add(&self, mut task: Task) -> Result<(), String> {
        task.id = uuid::Uuid::new_v4().to_string();
        task.next_trigger = Some(calculate_next_trigger(&task));
        let mut tasks = self.tasks.lock().unwrap();
        tasks.push(task);
        drop(tasks);
        self.save()
    }

    pub fn update(&self, task: Task) -> Result<(), String> {
        let mut tasks = self.tasks.lock().unwrap();
        if let Some(existing) = tasks.iter_mut().find(|t| t.id == task.id) {
            *existing = task;
            existing.next_trigger = Some(calculate_next_trigger(existing));
        } else {
            return Err("Task not found".to_string());
        }
        drop(tasks);
        self.save()
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let mut tasks = self.tasks.lock().unwrap();
        let len_before = tasks.len();
        tasks.retain(|t| t.id != id);
        if tasks.len() == len_before {
            return Err("Task not found".to_string());
        }
        drop(tasks);
        self.save()
    }

    pub fn get_tasks_for_date(&self, date: NaiveDate) -> Vec<TaskWithNextTrigger> {
        let tasks = self.tasks.lock().unwrap();
        let weekday_num = weekday_to_number(date.weekday());

        tasks
            .iter()
            .filter(|task| match task.task_type.as_str() {
                "daily" => true,
                "weekly" => task
                    .days_of_week
                    .as_ref()
                    .map(|days| days.contains(&weekday_num))
                    .unwrap_or(false),
                "once" => task
                    .once_date
                    .as_ref()
                    .map(|d| d == &date.format("%Y-%m-%d").to_string())
                    .unwrap_or(false),
                _ => false,
            })
            .map(TaskWithNextTrigger::from)
            .collect()
    }

    pub fn get_tasks_needing_trigger(&self) -> Vec<(Task, Config)> {
        let now = Local::now();
        let config = self.get_config();
        let tasks = self.tasks.lock().unwrap();
        tasks
            .iter()
            .filter(|task| task.next_trigger.map(|nt| nt <= now).unwrap_or(false))
            .map(|task| (task.clone(), config.clone()))
            .collect()
    }

    pub fn mark_as_triggered(&self, task_id: &str) -> Result<(), String> {
        let mut tasks = self.tasks.lock().unwrap();
        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
            match task.task_type.as_str() {
                "daily" => {
                    task.next_trigger = Some(calculate_next_trigger(task));
                }
                "weekly" => {
                    task.next_trigger = Some(calculate_next_trigger(task));
                }
                "once" => {
                    tasks.retain(|t| t.id != task_id);
                }
                _ => {}
            }
        }
        drop(tasks);
        self.save()
    }

    pub fn load_config(&self) -> Result<Config, String> {
        let config_path = self.config_path();
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .map_err(|e| format!("Failed to read config.json: {}", e))?;

            let parsed: Result<Config, _> = serde_json::from_str(&content);

            match parsed {
                Ok(config) => Ok(config),
                Err(_) => {
                    let old_config: OldConfigFormat = serde_json::from_str(&content)
                        .map_err(|e| format!("Failed to parse config.json: {}", e))?;
                    let new_config = migrate_old_config(&old_config);
                    self.save_config(&new_config)?;
                    Ok(new_config)
                }
            }
        } else {
            let default_config = Config::default();
            self.save_config(&default_config)?;
            Ok(default_config)
        }
    }

    pub fn save_config(&self, config: &Config) -> Result<(), String> {
        let config_path = self.config_path();
        let content = serde_json::to_string_pretty(config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        fs::write(&config_path, content)
            .map_err(|e| format!("Failed to write config.json: {}", e))?;
        let mut cache = self.config_cache.lock().unwrap();
        *cache = Some(config.clone());
        Ok(())
    }

    fn tasks_path(&self) -> PathBuf {
        self.get_exe_dir().join("tasks.json")
    }

    fn config_path(&self) -> PathBuf {
        self.get_exe_dir().join("config.json")
    }

    fn recalculate_all_next_triggers(&self) {
        let mut tasks = self.tasks.lock().unwrap();
        for task in tasks.iter_mut() {
            task.next_trigger = Some(calculate_next_trigger(task));
        }
    }
}

pub fn calculate_next_trigger(task: &Task) -> DateTime<Local> {
    let now = Local::now();
    let today = now.date_naive();

    let (hour, minute) = parse_time(&task.time);

    match task.task_type.as_str() {
        "daily" => {
            let today_naive = NaiveDate::from_ymd_opt(today.year(), today.month(), today.day());
            if let Some(nd) = today_naive {
                let today_time = nd.and_hms_opt(hour, minute, 0).unwrap();
                let today_trigger = today_time.and_local_timezone(Local).unwrap();
                if today_trigger > now {
                    return today_trigger;
                }
            }

            let tomorrow = today.succ_opt().unwrap();
            let tomorrow_naive =
                NaiveDate::from_ymd_opt(tomorrow.year(), tomorrow.month(), tomorrow.day());
            if let Some(nd) = tomorrow_naive {
                let tomorrow_time = nd.and_hms_opt(hour, minute, 0).unwrap();
                return tomorrow_time.and_local_timezone(Local).unwrap();
            }

            now
        }
        "weekly" => {
            let days = task.days_of_week.as_ref().cloned().unwrap_or_default();
            if days.is_empty() {
                eprintln!("Warning: weekly task {} has empty days_of_week", task.id);
                return now;
            }

            let mut candidate_dates: Vec<NaiveDate> = Vec::new();
            let mut current = today;
            for _ in 0..7 {
                if days.contains(&weekday_to_number(current.weekday())) {
                    candidate_dates.push(current);
                }
                current = current.succ_opt().unwrap();
            }

            candidate_dates.sort();

            for candidate in candidate_dates {
                if let Some(t) = candidate.and_hms_opt(hour, minute, 0) {
                    let trigger = t.and_local_timezone(Local).unwrap();
                    if trigger > now {
                        return trigger;
                    }
                }
            }

            let next_week_day = days[0];
            let mut next_week_date = today;
            while next_week_date.weekday() != number_to_weekday(next_week_day) {
                if let Some(d) = next_week_date.succ_opt() {
                    next_week_date = d;
                } else {
                    break;
                }
            }
            for _ in 0..7 {
                if let Some(d) = next_week_date.succ_opt() {
                    next_week_date = d;
                }
            }

            if let Some(t) = next_week_date.and_hms_opt(hour, minute, 0) {
                return t.and_local_timezone(Local).unwrap();
            }

            now
        }
        "once" => {
            if let Some(ref date_str) = task.once_date {
                if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                    if let Some(t) = date.and_hms_opt(hour, minute, 0) {
                        return t.and_local_timezone(Local).unwrap();
                    }
                }
            }
            now
        }
        _ => now,
    }
}

fn parse_time(time_str: &str) -> (u32, u32) {
    let parts: Vec<&str> = time_str.split(':').collect();
    let hour: u32 = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let minute: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    (hour, minute)
}

fn weekday_to_number(wd: Weekday) -> u8 {
    match wd {
        Weekday::Mon => 1,
        Weekday::Tue => 2,
        Weekday::Wed => 3,
        Weekday::Thu => 4,
        Weekday::Fri => 5,
        Weekday::Sat => 6,
        Weekday::Sun => 7,
    }
}

fn number_to_weekday(num: u8) -> Weekday {
    match num {
        1 => Weekday::Mon,
        2 => Weekday::Tue,
        3 => Weekday::Wed,
        4 => Weekday::Thu,
        5 => Weekday::Fri,
        6 => Weekday::Sat,
        7 => Weekday::Sun,
        _ => Weekday::Mon,
    }
}

#[derive(Serialize, Deserialize)]
struct OldConfigFormat {
    command: String,
}

fn migrate_old_config(old: &OldConfigFormat) -> Config {
    let parts: Vec<&str> = old.command.split_whitespace().collect();
    let exe = parts.first().unwrap_or(&"claude").to_string();
    let mut args: Vec<CliArg> = Vec::new();
    let mut i = 1;
    while i < parts.len() {
        let name = parts[i].to_string();
        if i + 1 < parts.len() && !parts[i + 1].starts_with('-') {
            args.push(CliArg {
                name,
                value: parts[i + 1].to_string(),
            });
            i += 2;
        } else {
            args.push(CliArg {
                name,
                value: "".to_string(),
            });
            i += 1;
        }
    }
    Config { command: exe, args }
}
