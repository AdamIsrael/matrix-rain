use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use matrix::{CharSet, MatrixConfig, MatrixRain, MatrixRainState, Theme};

#[derive(Parser, Debug)]
#[command(version, about = "Matrix digital rain effect for the terminal.")]
struct Cli {
    /// Frames per second.
    #[arg(short = 'f', long, default_value_t = 30, value_parser = clap::value_parser!(u16).range(1..))]
    fps: u16,

    /// Speed multiplier.
    #[arg(short = 's', long, default_value_t = 1.0, value_parser = parse_positive_f32)]
    speed: f32,

    /// Column density (0.0–1.0).
    #[arg(short = 'd', long, default_value_t = 0.6, value_parser = parse_density)]
    density: f32,

    /// Character set.
    #[arg(long, value_enum, default_value_t = CharsetArg::Matrix)]
    charset: CharsetArg,

    /// Theme.
    #[arg(long, value_enum, default_value_t = ThemeArg::Green)]
    theme: ThemeArg,

    /// Disable the classic white head.
    #[arg(long)]
    no_head_white: bool,

    /// Disable bold head cells (bold is on by default).
    #[arg(long)]
    no_bold: bool,

    /// Deterministic RNG seed.
    #[arg(long)]
    seed: Option<u64>,

    /// Exit on any keypress (default: q/Esc/Ctrl-C only).
    #[arg(short = 'q', long)]
    quit_on_any_key: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CharsetArg {
    Matrix,
}

impl From<CharsetArg> for CharSet {
    fn from(c: CharsetArg) -> Self {
        match c {
            CharsetArg::Matrix => CharSet::Matrix,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ThemeArg {
    Green,
}

impl From<ThemeArg> for Theme {
    fn from(t: ThemeArg) -> Self {
        match t {
            ThemeArg::Green => Theme::ClassicGreen,
        }
    }
}

fn parse_positive_f32(s: &str) -> Result<f32, String> {
    let v: f32 = s.parse().map_err(|e: std::num::ParseFloatError| e.to_string())?;
    if !v.is_finite() || v <= 0.0 {
        return Err(format!("must be a positive finite number (got {v})"));
    }
    Ok(v)
}

fn parse_density(s: &str) -> Result<f32, String> {
    let v: f32 = s.parse().map_err(|e: std::num::ParseFloatError| e.to_string())?;
    if !v.is_finite() || !(0.0..=1.0).contains(&v) {
        return Err(format!("must be a finite number in [0.0, 1.0] (got {v})"));
    }
    Ok(v)
}

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

#[cfg(unix)]
fn install_signal_handlers(flag: Arc<AtomicBool>) -> io::Result<()> {
    use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
    signal_hook::flag::register(SIGINT, flag.clone())?;
    signal_hook::flag::register(SIGTERM, flag.clone())?;
    signal_hook::flag::register(SIGHUP, flag)?;
    Ok(())
}

#[cfg(not(unix))]
fn install_signal_handlers(_flag: Arc<AtomicBool>) -> io::Result<()> {
    Ok(())
}

fn should_quit(key: &KeyEvent, any_key: bool) -> bool {
    if matches!(key.kind, KeyEventKind::Release | KeyEventKind::Repeat) {
        return false;
    }
    if any_key {
        return true;
    }
    if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
        return true;
    }
    matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C'))
        && key.modifiers.contains(KeyModifiers::CONTROL)
}

fn build_config(args: &Cli) -> Result<MatrixConfig> {
    MatrixConfig::builder()
        .fps(args.fps)
        .speed(args.speed)
        .density(args.density)
        .charset(args.charset.into())
        .theme(args.theme.into())
        .head_white(!args.no_head_white)
        .bold_head(!args.no_bold)
        .build()
        .map_err(|e| anyhow::anyhow!(e.to_string()))
}

fn run(args: Cli, shutdown: Arc<AtomicBool>) -> Result<()> {
    let cfg = build_config(&args).context("invalid configuration")?;

    let mut state = match args.seed {
        Some(s) => MatrixRainState::with_seed(s),
        None => MatrixRainState::new(),
    };

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend).context("creating terminal")?;

    let poll_dur = Duration::from_millis((1000u64 / cfg.fps as u64).max(1));

    while !shutdown.load(Ordering::Relaxed) {
        match terminal.draw(|f| {
            f.render_stateful_widget(MatrixRain::new(&cfg), f.size(), &mut state);
        }) {
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => return Ok(()),
            Err(e) => return Err(e).context("drawing frame"),
        }

        match event::poll(poll_dur) {
            Ok(true) => match event::read().context("reading terminal event")? {
                Event::Key(key) => {
                    if should_quit(&key, args.quit_on_any_key) {
                        break;
                    }
                }
                _ => {}
            },
            Ok(false) => {}
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => return Ok(()),
            Err(e) => return Err(e).context("polling for events"),
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let args = Cli::parse();

    if !io::stdout().is_terminal() {
        let _ = writeln!(
            io::stderr(),
            "matrix: stdout is not a terminal; refusing to start"
        );
        std::process::exit(2);
    }

    install_panic_hook();
    let shutdown = Arc::new(AtomicBool::new(false));
    install_signal_handlers(shutdown.clone()).context("installing signal handlers")?;

    let _guard = TerminalGuard::enter().context("entering raw mode")?;
    run(args, shutdown)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_help_renders_without_panic() {
        let _ = Cli::command().render_help().to_string();
    }

    #[test]
    fn cli_default_parse() {
        let cli = Cli::try_parse_from(["matrix"]).unwrap();
        assert_eq!(cli.fps, 30);
        assert_eq!(cli.speed, 1.0);
        assert_eq!(cli.density, 0.6);
        assert!(!cli.no_head_white);
        assert!(!cli.no_bold);
        assert_eq!(cli.seed, None);
        assert!(!cli.quit_on_any_key);
    }

    #[test]
    fn cli_rejects_zero_fps() {
        assert!(Cli::try_parse_from(["matrix", "--fps", "0"]).is_err());
    }

    #[test]
    fn cli_rejects_negative_speed() {
        assert!(Cli::try_parse_from(["matrix", "--speed", "-1.0"]).is_err());
    }

    #[test]
    fn cli_rejects_zero_speed() {
        assert!(Cli::try_parse_from(["matrix", "--speed", "0"]).is_err());
    }

    #[test]
    fn cli_rejects_density_above_one() {
        assert!(Cli::try_parse_from(["matrix", "--density", "1.5"]).is_err());
    }

    #[test]
    fn cli_rejects_density_below_zero() {
        assert!(Cli::try_parse_from(["matrix", "--density", "-0.1"]).is_err());
    }

    #[test]
    fn cli_density_zero_and_one_accepted() {
        Cli::try_parse_from(["matrix", "--density", "0.0"]).unwrap();
        Cli::try_parse_from(["matrix", "--density", "1.0"]).unwrap();
    }

    #[test]
    fn cli_seed_parses_u64() {
        let cli = Cli::try_parse_from(["matrix", "--seed", "1234567890"]).unwrap();
        assert_eq!(cli.seed, Some(1234567890));
    }

    #[test]
    fn cli_no_head_white_and_no_bold_flags() {
        let cli = Cli::try_parse_from(["matrix", "--no-head-white", "--no-bold"]).unwrap();
        assert!(cli.no_head_white);
        assert!(cli.no_bold);
    }

    #[test]
    fn cli_quit_on_any_key_short_and_long() {
        Cli::try_parse_from(["matrix", "-q"]).unwrap();
        Cli::try_parse_from(["matrix", "--quit-on-any-key"]).unwrap();
    }

    #[test]
    fn build_config_inverts_no_head_white_and_no_bold() {
        let args = Cli::try_parse_from(["matrix", "--no-head-white", "--no-bold"]).unwrap();
        let cfg = build_config(&args).unwrap();
        assert!(!cfg.head_white);
        assert!(!cfg.bold_head);
    }

    #[test]
    fn build_config_defaults_have_white_head_and_bold() {
        let args = Cli::try_parse_from(["matrix"]).unwrap();
        let cfg = build_config(&args).unwrap();
        assert!(cfg.head_white);
        assert!(cfg.bold_head);
    }

    fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, mods)
    }

    #[test]
    fn quit_on_q() {
        assert!(should_quit(&key(KeyCode::Char('q'), KeyModifiers::NONE), false));
    }

    #[test]
    fn quit_on_esc() {
        assert!(should_quit(&key(KeyCode::Esc, KeyModifiers::NONE), false));
    }

    #[test]
    fn quit_on_ctrl_c() {
        assert!(should_quit(
            &key(KeyCode::Char('c'), KeyModifiers::CONTROL),
            false
        ));
    }

    #[test]
    fn does_not_quit_on_other_keys_default() {
        assert!(!should_quit(&key(KeyCode::Char('x'), KeyModifiers::NONE), false));
        assert!(!should_quit(&key(KeyCode::Enter, KeyModifiers::NONE), false));
    }

    #[test]
    fn quit_on_any_key_quits_on_any_key() {
        assert!(should_quit(&key(KeyCode::Char('x'), KeyModifiers::NONE), true));
        assert!(should_quit(&key(KeyCode::Enter, KeyModifiers::NONE), true));
    }

    #[test]
    fn release_and_repeat_events_are_ignored() {
        let mut k = key(KeyCode::Char('q'), KeyModifiers::NONE);
        k.kind = KeyEventKind::Release;
        assert!(!should_quit(&k, true));
        k.kind = KeyEventKind::Repeat;
        assert!(!should_quit(&k, true));
    }

    #[test]
    fn parse_positive_f32_rejects_nan() {
        assert!(parse_positive_f32("NaN").is_err());
    }

    #[test]
    fn parse_positive_f32_rejects_infinity() {
        assert!(parse_positive_f32("inf").is_err());
    }

    #[test]
    fn parse_density_rejects_nan() {
        assert!(parse_density("NaN").is_err());
    }
}
