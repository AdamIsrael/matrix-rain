//! Embed the Matrix rain widget with a user-supplied character set.
//!
//! Run with: `cargo run --example custom_charset`
//! Quit: q, Esc, or Ctrl-C.

use std::io;
use std::time::Duration;

use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use matrix_rain::{CharSet, MatrixConfig, MatrixRain, MatrixRainState};

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let guard = Self;
        execute!(io::stdout(), EnterAlternateScreen, Hide)?;
        Ok(guard)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), Show, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(io::stdout(), Show, LeaveAlternateScreen);
        let _ = disable_raw_mode();
        prev(info);
    }));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    install_panic_hook();
    let _guard = TerminalGuard::enter()?;

    // Mix katakana with hex digits — every char must be exactly one terminal cell wide
    // (full-width / combining chars are not detected per spec §5.4).
    let chars: Vec<char> = "ｦｧｨｩｪｫｬｭｮｯｱｲｳｴｵ0123456789abcdef".chars().collect();

    let cfg = MatrixConfig::builder()
        .charset(CharSet::Custom(chars))
        .density(0.7)
        .build()?;
    let mut state = MatrixRainState::new();
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let poll_dur = Duration::from_millis((1000u64 / cfg.fps as u64).max(1));

    loop {
        terminal.draw(|f| {
            f.render_stateful_widget(MatrixRain::new(&cfg), f.size(), &mut state);
        })?;

        if event::poll(poll_dur)? {
            if let Event::Key(key) = event::read()? {
                if matches!(key.kind, KeyEventKind::Release | KeyEventKind::Repeat) {
                    continue;
                }
                if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
                    break;
                }
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
                {
                    break;
                }
            }
        }
    }

    Ok(())
}
