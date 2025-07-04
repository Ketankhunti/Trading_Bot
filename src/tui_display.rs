// src/tui_display.rs

//! This module provides a generic function to display any struct that implements
//! the `Debug` trait within a simple `ratatui` Text User Interface (TUI).

use std::{
    io::{self, stdout},
    fmt::Debug,
    time::Duration,
};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};

/// Sets up the terminal for TUI mode.
fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>, Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restores the terminal after TUI mode.
fn restore_terminal(mut terminal: Terminal<CrosstermBackend<io::Stdout>>) -> Result<(), Box<dyn std::error::Error>> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

/// Draws the UI for displaying the struct's debug output.
fn ui<T: Debug>(frame: &mut Frame, item: &T, title: &str, scroll: u16) {
    let size = frame.size();

    // Create a central block for the content
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(size);

    // Format the debug output of the item
    let debug_output = format!("{:#?}", item);
    let total_lines = debug_output.lines().count() as u16;

    // Create a paragraph widget to display the debug output
    let paragraph = Paragraph::new(debug_output)
        .block(
            Block::default()
                .title(Line::from(vec![
                    Span::styled(" ", Style::default()),
                    Span::styled(title, Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(" ", Style::default()),
                    Span::styled("(q: quit, ↑/↓: scroll)", Style::default().italic()),
                ]))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ratatui::style::Color::Blue))
        )
        .wrap(ratatui::widgets::Wrap { trim: false })
        .scroll((scroll, 0));

    frame.render_widget(paragraph, chunks[0]);
}

/// Displays any struct that implements `Debug` in a `ratatui` terminal UI.
///
/// The UI will display the pretty-printed debug output of the struct.
/// Press 'q' to quit the display.
///
/// # Arguments
/// * `item` - A reference to the struct to be displayed.
/// * `title` - A title to display at the top of the TUI window.
pub async fn display_struct_in_tui<T: Debug>(item: &T, title: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = setup_terminal()?;
    let mut scroll: u16 = 0;
    let debug_output = format!("{:#?}", item);
    let total_lines = debug_output.lines().count() as u16;

    loop {
        let mut visible_height = 0;
        terminal.draw(|frame| {
            visible_height = frame.size().height.saturating_sub(2); // Subtract borders
            ui(frame, item, title, scroll);
        })?;

        let max_scroll = total_lines.saturating_sub(visible_height);
        if scroll > max_scroll {
            scroll = max_scroll;
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Up => {
                        if scroll > 0 { scroll -= 1; }
                    }
                    KeyCode::Down => {
                        if scroll < max_scroll { scroll += 1; }
                    }
                    KeyCode::PageUp => {
                        scroll = scroll.saturating_sub(10);
                    }
                    KeyCode::PageDown => {
                        scroll = (scroll + 10).min(max_scroll);
                    }
                    _ => {}
                }
            }
        }
    }

    restore_terminal(terminal)?;
    Ok(())
}
