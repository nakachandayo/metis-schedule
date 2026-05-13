use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct CliArg {
    pub name: String,
    #[serde(default)]
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CliCommand {
    pub exe: String,
    #[serde(default)]
    pub args: Vec<CliArg>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: String,
    #[serde(rename = "task_type")]
    pub task_type: String,
    pub time: String,
    #[serde(rename = "days_of_week", skip_serializing_if = "Option::is_none")]
    pub days_of_week: Option<Vec<u8>>,
    #[serde(rename = "once_date", skip_serializing_if = "Option::is_none")]
    pub once_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cli_command: Option<CliCommand>,
    #[serde(skip)]
    pub next_trigger: Option<DateTime<Local>>,
}

impl Task {
    pub fn new(task_type: String, time: String, prompt: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            task_type,
            time,
            days_of_week: None,
            once_date: None,
            label: None,
            prompt,
            cli_command: None,
            next_trigger: None,
        }
    }

    pub fn with_daily(time: String, prompt: String) -> Self {
        let mut task = Self::new("daily".to_string(), time, prompt);
        task.label = Some("每日任务".to_string());
        task
    }

    pub fn with_weekly(time: String, days_of_week: Vec<u8>, prompt: String) -> Self {
        let mut task = Self::new("weekly".to_string(), time, prompt);
        task.days_of_week = Some(days_of_week);
        task.label = Some("每周任务".to_string());
        task
    }

    pub fn with_once(time: String, once_date: String, prompt: String) -> Self {
        let mut task = Self::new("once".to_string(), time, prompt);
        task.once_date = Some(once_date);
        task.label = Some("一次性任务".to_string());
        task
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TaskWithNextTrigger {
    pub id: String,
    #[serde(rename = "task_type")]
    pub task_type: String,
    pub time: String,
    #[serde(rename = "days_of_week", skip_serializing_if = "Option::is_none")]
    pub days_of_week: Option<Vec<u8>>,
    #[serde(rename = "once_date", skip_serializing_if = "Option::is_none")]
    pub once_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cli_command: Option<CliCommand>,
    #[serde(rename = "next_trigger")]
    pub next_trigger: String,
}

impl From<&Task> for TaskWithNextTrigger {
    fn from(task: &Task) -> Self {
        Self {
            id: task.id.clone(),
            task_type: task.task_type.clone(),
            time: task.time.clone(),
            days_of_week: task.days_of_week.clone(),
            once_date: task.once_date.clone(),
            label: task.label.clone(),
            prompt: task.prompt.clone(),
            cli_command: task.cli_command.clone(),
            next_trigger: task
                .next_trigger
                .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string())
                .unwrap_or_default(),
        }
    }
}
