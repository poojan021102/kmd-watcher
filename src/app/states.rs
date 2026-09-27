#[derive(Clone)]
pub struct AppState {
    pub current_execution_state: ExecutionState,
    pub current_displayed_command: String,
}

#[derive(Clone)]
pub struct RunnerState {
    pub current_command: String,
    pub current_response: String,
}

#[derive(Clone)]
pub enum ExecutionState {
    Executing(RunnerState),
    Killed,
    Idle,
}

impl ExecutionState {
    pub fn from(command: String) -> Self {
        Self::Executing(RunnerState {
            current_command: command,
            current_response: String::new(),
        })
    }

    pub fn kill_execution(&self) -> Self {
        Self::Killed
    }
    pub fn make_idle(&self) -> Self {
        Self::Idle
    }
}

impl AppState {
    pub fn default() -> Self {
        Self {
            current_execution_state: ExecutionState::Idle,
            current_displayed_command: String::new(),
        }
    }

    pub fn append_new_char(&mut self, new_char: char) {
        self.current_displayed_command.push(new_char);
    }

    pub fn remove_last_char(&mut self) {
        self.current_displayed_command.pop();
    }

    pub fn check_is_last_slash(&self) -> bool {
        for c in self.current_displayed_command.chars().rev() {
            if c == '/' {
                return true;
            } else if c == ' ' {
                continue;
            } else {
                return false;
            }
        }
        return false;
    }

    pub fn start_execution(&mut self) {
        self.current_execution_state = ExecutionState::from(self.current_displayed_command.clone())
    }

    pub fn kill_execution(&mut self) {
        self.current_execution_state = self.current_execution_state.kill_execution();
    }
    pub fn make_idle(&mut self) {
        self.current_execution_state = self.current_execution_state.make_idle();
    }
}
