mod ui;
mod state;
mod data;
mod map_draw;
mod gdp_reader;

use crossterm::{
    event::{self, Event, KeyEvent, KeyEventKind, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use state::AppState;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load application state with GDP data
    let mut state = AppState::new("data")?;

    // Enter raw mode and alternate screen
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop: draw UI and handle key input
    loop {
        terminal.draw(|f| ui::draw(f, &mut state))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(KeyEvent { code, kind: KeyEventKind::Press, .. }) = event::read()? {
                if state.handle_input(code) {
                    break; // Exit on quit command
                }
            }
        }
    }

    // Restore terminal state
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
