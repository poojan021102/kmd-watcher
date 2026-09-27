use crate::app::states::{AppState, ExecutionState};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use ratatui_textarea::TextArea;
use std::sync::RwLockReadGuard;

pub fn show_ui(
    frame: &mut Frame,
    app_state: RwLockReadGuard<'_, AppState>,
    command_text_area: &TextArea,
    stdout: Vec<String>,
    stderr: Vec<String>,
    selected_line: u16,
) {
    // Split the layout into two vertical direction
    let chunks = Layout::vertical([Constraint::Length(4), Constraint::Min(0)]).split(frame.area());
    // Render the text area
    frame.render_widget(command_text_area, chunks[0]);

    let visible_lines = chunks[1].height - 3;

    // Render other half
    match app_state.current_execution_state {
        ExecutionState::Executing(_) => {
            let para = get_para(&stdout, &stderr, selected_line, visible_lines);
            frame.render_widget(para, chunks[1]);
        }
        _ => {
            frame.render_widget(Clear, chunks[1]);
        }
    };
}

pub fn get_para<'a>(
    stdout: &'a Vec<String>,
    stderr: &'a Vec<String>,
    selected_line: u16,
    visible_lines: u16,
) -> Paragraph<'a> {
    let selected_line: usize = selected_line.into();
    let visible_lines: usize = visible_lines.into();

    let mut stdout_lines: Vec<Line> = stdout
        .iter()
        .enumerate()
        .map(|(index, stdout_line)| {
            if index == selected_line {
                return Line::from(stdout_line.as_str())
                    .style(Style::new().bg(Color::White).black());
            }
            Line::from(stdout_line.as_str())
        })
        .collect();
    let stdout_len = stdout_lines.len();

    let mut stderr_lines: Vec<Line> = stderr
        .iter()
        .enumerate()
        .map(|(index, stderr_line)| {
            if (selected_line as isize) - (stdout_len as isize) == (index as isize) {
                return Line::from(stderr_line.as_str()).style(Style::new().bg(Color::Red).white());
            }
            Line::from(stderr_line.as_str()).style(Style::new().red())
        })
        .collect();

    stdout_lines.append(&mut stderr_lines);

    let mut scroll_offset = 0;
    if selected_line > visible_lines {
        scroll_offset = selected_line - visible_lines;
    }

    Paragraph::new(stdout_lines)
        .block(
            Block::new()
                .title("Command Response")
                .borders(Borders::all()),
        )
        .scroll((scroll_offset.try_into().unwrap(), 0))
        .wrap(Wrap { trim: false })
}
