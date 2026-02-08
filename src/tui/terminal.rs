//! Terminal setup and teardown utilities.

use std::io::{self, IsTerminal, Stdout};

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::Result;

/// Type alias for our terminal backend.
pub type Tui = Terminal<CrosstermBackend<Stdout>>;

/// Initializes the terminal for TUI rendering.
///
/// Enables raw mode and switches to the alternate screen buffer.
/// Returns a configured Terminal instance.
///
/// # Errors
///
/// Returns an error if terminal initialization fails or if stdout is not a TTY.
pub fn setup_terminal() -> Result<Tui> {
    // Check if stdout is a terminal
    if !io::stdout().is_terminal() {
        return Err(crate::LeesonError::Io(
            "TUI requires an interactive terminal (TTY). Cannot run in a non-interactive environment.".to_string()
        ));
    }

    enable_raw_mode()
        .map_err(|e| crate::LeesonError::Io(format!("failed to enable raw mode: {e}")))?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(|e| {
        // Try to restore terminal state before returning error
        let _ = disable_raw_mode();
        crate::LeesonError::Io(format!("failed to enter alternate screen: {e}"))
    })?;

    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend).map_err(|e| {
        // Try to restore terminal state before returning error
        let _ = disable_raw_mode();
        crate::LeesonError::Io(format!("failed to create terminal: {e}"))
    })?;

    Ok(terminal)
}

/// Restores the terminal to its original state.
///
/// Disables raw mode and returns to the main screen buffer.
///
/// # Errors
///
/// Returns an error if terminal restoration fails.
pub fn restore_terminal(terminal: &mut Tui) -> Result<()> {
    disable_raw_mode().map_err(|e| crate::LeesonError::Io(e.to_string()))?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .map_err(|e| crate::LeesonError::Io(e.to_string()))?;
    terminal
        .show_cursor()
        .map_err(|e| crate::LeesonError::Io(e.to_string()))?;
    Ok(())
}
