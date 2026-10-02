pub mod app;
pub mod ui;

use std::{
    io::{self, BufRead, BufReader},
    process::{Command, Stdio},
    sync::{Arc, Mutex, RwLock},
    thread::{self, sleep},
    time::Duration,
};

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    Terminal,
    backend::Backend,
    style::{Color, Style},
    widgets::{Block, Borders},
};
use ratatui_textarea::TextArea;

use crate::app::states::{AppState, ExecutionState};
use crate::ui::show_ui;

fn run_in_background(
    state_ref: Arc<RwLock<AppState>>,
    stdout_output: Arc<RwLock<Vec<String>>>,
    stderr_output: Arc<RwLock<Vec<String>>>,
    child_process: Arc<RwLock<Option<u32>>>,
) {
    loop {
        // Sleep at 2 second duration
        let app_state = state_ref.read().unwrap().clone();

        if let ExecutionState::Executing(execution_state) = app_state.current_execution_state {
            // Now create a new process for call this command

            let cmd = if cfg!(target_os = "windows") {
                "cmd"
            } else {
                "sh"
            };

            let shell_arg = if cfg!(target_os = "windows") {
                "/C"
            } else {
                "-c"
            };
            let Ok(cwd) = std::env::current_dir() else {
                continue;
            };

            let mut child_result = Command::new(cmd)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .args([shell_arg, &execution_state.current_command])
                .current_dir(cwd)
                .spawn();

            let local_stdout = Arc::new(Mutex::new(Vec::new()));
            let local_stderr = Arc::new(Mutex::new(Vec::new()));

            match &mut child_result {
                Ok(child) => {
                    {
                        let mut opt = child_process.write().unwrap();
                        *opt = Some(child.id());
                    }
                    // Read the stdout and stderr
                    let stdout = child.stdout.take().expect("Not able to get child stdout");
                    let stderr = child.stderr.take().expect("Not able to get child stderr");

                    let local_stdout_clone = local_stdout.clone();
                    let local_stderr_clone = local_stderr.clone();

                    let stdout_thread = thread::spawn(move || {
                        // Read the stdout thread
                        let reader = BufReader::new(stdout);
                        for line in reader.lines() {
                            if let Ok(l) = line {
                                let mut lock = local_stdout_clone.lock().unwrap();
                                lock.push(l);
                            }
                        }
                    });
                    let stderr_thread = thread::spawn(move || {
                        // Read the stderr thread
                        let reader = BufReader::new(stderr);
                        for line in reader.lines() {
                            if let Ok(l) = line {
                                let mut lock = local_stderr_clone.lock().unwrap();
                                lock.push(l);
                            }
                        }
                    });

                    let _ = stdout_thread.join();
                    let _ = stderr_thread.join();
                }
                Err(e) => {
                    // send the error
                    let mut lock = local_stderr.lock().unwrap();
                    lock.push(e.to_string());
                }
            }

            // Use this new value to the string
            let mut stdout_new = stdout_output.write().unwrap();
            stdout_new.clear();
            stdout_new.clone_from(&local_stdout.lock().unwrap());
            let mut stderr_new = stderr_output.write().unwrap();
            stderr_new.clear();
            stderr_new.clone_from(&local_stderr.lock().unwrap());
        }

        sleep(Duration::from_secs(2));
    }
}

fn kill_process(pid: u32) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status()?;
    }

    #[cfg(windows)]
    {
        Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .status()?;
    }

    Ok(())
}

pub fn run_app<B: Backend>(backend: &mut Terminal<B>, app_state: AppState) -> io::Result<()>
where
    io::Error: From<B::Error>,
{
    let threaded_state = Arc::new(RwLock::new(app_state));

    let bg_state_ref = Arc::clone(&threaded_state);

    let stdout_shared = Arc::new(RwLock::new(Vec::new()));
    let stderr_shared = Arc::new(RwLock::new(Vec::new()));

    let stdout_cloned = stdout_shared.clone();
    let stderr_cloned = stderr_shared.clone();

    // Reference of child process
    let child_process = Arc::new(RwLock::new(None));
    let child_process_clone = child_process.clone();
    // Run background execution thread
    thread::spawn(move || {
        run_in_background(
            bg_state_ref,
            stdout_cloned,
            stderr_cloned,
            child_process_clone,
        )
    });

    // Create a new reference for writing state here
    let state_ref = Arc::clone(&threaded_state);

    // Rendef a responsive text area
    let mut command_text_area = TextArea::default();
    // let mut result_text_area = TextArea::default();

    let mut selected_line = 0;

    // Loop through to read the key events
    loop {
        if stderr_shared.read().unwrap().len() + stdout_shared.read().unwrap().len()
            <= (selected_line) as usize
        {
            selected_line =
                (stderr_shared.read().unwrap().len() + stdout_shared.read().unwrap().len() - 1)
                    .try_into()
                    .unwrap_or(0);
        }
        // Render in UI
        backend.draw(|frame| {
            show_ui(
                frame,
                state_ref.read().unwrap(),
                &command_text_area,
                // &mut result_text_area,
                stdout_shared.read().unwrap().clone(),
                stderr_shared.read().unwrap().clone(),
                selected_line,
            )
        })?;

        // dynamically update the text-area styling

        match state_ref.read().unwrap().current_execution_state {
            ExecutionState::Executing(_) => {
                command_text_area.set_block(
                    Block::default()
                        .title("Executing your command, Press ESC to stop")
                        .borders(Borders::all())
                        .border_style(Style::default().fg(Color::Green)),
                );
            }
            ExecutionState::Killed => {
                command_text_area.set_block(
                    Block::default()
                        .title("Stopping your command, please wait")
                        .borders(Borders::all())
                        .border_style(Style::default().fg(Color::Red)),
                );
            }
            _ => {
                command_text_area.set_block(
                    Block::default()
                        .title("Type your command, Press ESC to stop")
                        .borders(Borders::all())
                        .border_style(Style::default().fg(Color::Yellow)),
                );
            }
        }

        // Read the event using polling
        if event::poll(Duration::from_millis(500))? {
            match event::read()? {
                Event::Key(key_event) => {
                    // Press of the any character
                    match key_event.code {
                        KeyCode::Up => {
                            let app_state = state_ref.read().unwrap();
                            if let ExecutionState::Executing(_) = app_state.current_execution_state
                            {
                                if let ExecutionState::Executing(_) =
                                    app_state.current_execution_state
                                {
                                    if let Some(result) = selected_line.checked_sub(1) {
                                        selected_line = result;
                                    }
                                }
                            }
                        }
                        KeyCode::Down => {
                            let app_state = state_ref.read().unwrap();
                            if let ExecutionState::Executing(_) = app_state.current_execution_state
                            {
                                if stderr_shared.read().unwrap().len()
                                    + stdout_shared.read().unwrap().len()
                                    > (selected_line + 1) as usize
                                {
                                    selected_line = selected_line + 1;
                                }
                            }
                        }
                        KeyCode::Esc => {
                            state_ref.write().unwrap().kill_execution();
                            stdout_shared.write().unwrap().clear();
                            stderr_shared.write().unwrap().clear();

                            let pid = child_process.read().unwrap();
                            if pid.is_some() {
                                let _ = kill_process(pid.unwrap());
                            }
                            state_ref.write().unwrap().make_idle();
                        }

                        KeyCode::Enter => {
                            let mut app_state = state_ref.write().unwrap();
                            if app_state.check_is_last_slash() {
                                if let ExecutionState::Idle = app_state.current_execution_state {
                                    command_text_area.input(key_event);
                                }
                            } else {
                                // Start execution
                                app_state.current_displayed_command =
                                    command_text_area.lines().join("\n");
                                app_state.start_execution();
                            }
                        }
                        KeyCode::Char('c') | KeyCode::Char('C')
                            if key_event.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            state_ref.write().unwrap().kill_execution();
                            stdout_shared.write().unwrap().clear();
                            stderr_shared.write().unwrap().clear();

                            let pid = child_process.read().unwrap();
                            if pid.is_some() {
                                let _ = kill_process(pid.unwrap());
                            }
                            state_ref.write().unwrap().make_idle();
                            break;
                        }
                        _ => {
                            let app_state = state_ref.read().unwrap();
                            if let ExecutionState::Idle = app_state.current_execution_state {
                                command_text_area.input(key_event);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}
