use crate::task::CliArg;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub command: String,
    #[serde(default)]
    pub args: Vec<CliArg>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            command: "claude".to_string(),
            args: vec![
                CliArg {
                    name: "-p".to_string(),
                    value: "".to_string(),
                },
                CliArg {
                    name: "--permission-mode".to_string(),
                    value: "acceptEdits".to_string(),
                },
            ],
        }
    }
}
