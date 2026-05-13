export interface CliArg {
  name: string;
  value: string;
}

export interface CliCommand {
  exe: string;
  args: CliArg[];
}

export interface Task {
  id: string;
  task_type: string;
  time: string;
  days_of_week: number[] | null;
  once_date: string | null;
  label: string | null;
  prompt: string;
  cli_command: CliCommand | null;
  next_trigger: string;
}

export interface Config {
  command: string;
  args: CliArg[];
}
