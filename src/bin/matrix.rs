use std::collections::HashSet;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
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

use matrix_rain::{CharSet, MatrixConfig, MatrixRain, MatrixRainState, Theme};

const MAX_CHARSET_FILE_BYTES: u64 = 1024 * 1024;

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
    #[arg(short = 'd', long, default_value_t = 0.6, value_parser = parse_unit_interval)]
    density: f32,

    /// Per-cell glyph reroll probability per tick (0.0–1.0).
    #[arg(long, default_value_t = 0.05, value_parser = parse_unit_interval)]
    mutation_rate: f32,

    /// Per-cell glitch (color flicker) probability per tick (0.0–1.0).
    #[arg(long, default_value_t = 0.0, value_parser = parse_unit_interval)]
    glitch: f32,

    /// Character set: one of matrix, ascii, hex, binary, or a path to a UTF-8 charset file (<= 1 MiB).
    #[arg(long, default_value = "matrix", value_parser = parse_charset_arg)]
    charset: CharsetSource,

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

#[derive(Clone, Debug)]
enum CharsetSource {
    Matrix,
    Ascii,
    Hex,
    Binary,
    Path(PathBuf),
}

fn parse_charset_arg(s: &str) -> Result<CharsetSource, String> {
    match s {
        "matrix" => Ok(CharsetSource::Matrix),
        "ascii" => Ok(CharsetSource::Ascii),
        "hex" => Ok(CharsetSource::Hex),
        "binary" => Ok(CharsetSource::Binary),
        other => {
            let path = PathBuf::from(other);
            if !path.exists() {
                return Err(format!(
                    "'{other}' is neither a built-in charset (matrix, ascii, hex, binary) nor an existing file path"
                ));
            }
            Ok(CharsetSource::Path(path))
        }
    }
}

fn resolve_charset(src: &CharsetSource) -> Result<CharSet> {
    match src {
        CharsetSource::Matrix => Ok(CharSet::Matrix),
        CharsetSource::Ascii => Ok(CharSet::Ascii),
        CharsetSource::Hex => Ok(CharSet::Hex),
        CharsetSource::Binary => Ok(CharSet::Binary),
        CharsetSource::Path(p) => Ok(CharSet::Custom(load_charset_from_path(p)?)),
    }
}

fn load_charset_from_path(path: &Path) -> Result<Vec<char>> {
    let meta = std::fs::metadata(path)
        .with_context(|| format!("reading metadata for {}", path.display()))?;
    if meta.len() > MAX_CHARSET_FILE_BYTES {
        anyhow::bail!(
            "charset file {} is {} bytes; maximum is {} ({}MiB)",
            path.display(),
            meta.len(),
            MAX_CHARSET_FILE_BYTES,
            MAX_CHARSET_FILE_BYTES / (1024 * 1024)
        );
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("reading {} as UTF-8", path.display()))?;
    let mut seen = HashSet::new();
    let mut chars = Vec::new();
    for c in content.chars() {
        if c.is_whitespace() || c.is_control() {
            continue;
        }
        if seen.insert(c) {
            chars.push(c);
        }
    }
    if chars.is_empty() {
        anyhow::bail!(
            "charset file {} contains no usable characters after filtering whitespace and controls",
            path.display()
        );
    }
    Ok(chars)
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ThemeArg {
    Green,
    Amber,
    Cyan,
    Red,
    Rainbow,
}

impl From<ThemeArg> for Theme {
    fn from(t: ThemeArg) -> Self {
        match t {
            ThemeArg::Green => Theme::ClassicGreen,
            ThemeArg::Amber => Theme::Amber,
            ThemeArg::Cyan => Theme::Cyan,
            ThemeArg::Red => Theme::Red,
            ThemeArg::Rainbow => Theme::Rainbow,
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

fn parse_unit_interval(s: &str) -> Result<f32, String> {
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
    let charset = resolve_charset(&args.charset)?;
    MatrixConfig::builder()
        .fps(args.fps)
        .speed(args.speed)
        .density(args.density)
        .mutation_rate(args.mutation_rate)
        .glitch(args.glitch)
        .charset(charset)
        .theme(args.theme.into())
        .head_white(!args.no_head_white)
        .bold_head(!args.no_bold)
        .build()
        .map_err(|e| anyhow::anyhow!(e.to_string()))
}

fn run(args: &Cli, cfg: &MatrixConfig, shutdown: Arc<AtomicBool>) -> Result<()> {
    let mut state = match args.seed {
        Some(s) => MatrixRainState::with_seed(s),
        None => MatrixRainState::new(),
    };

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend).context("creating terminal")?;

    let poll_dur = Duration::from_millis((1000u64 / cfg.fps as u64).max(1));

    while !shutdown.load(Ordering::Relaxed) {
        match terminal.draw(|f| {
            f.render_stateful_widget(MatrixRain::new(cfg), f.size(), &mut state);
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

    // Resolve config before entering raw mode so file/validation errors
    // exit cleanly without garbling the terminal.
    let cfg = match build_config(&args) {
        Ok(c) => c,
        Err(e) => {
            let _ = writeln!(io::stderr(), "matrix: {e:#}");
            std::process::exit(2);
        }
    };

    install_panic_hook();
    let shutdown = Arc::new(AtomicBool::new(false));
    install_signal_handlers(shutdown.clone()).context("installing signal handlers")?;

    let _guard = TerminalGuard::enter().context("entering raw mode")?;
    run(&args, &cfg, shutdown)
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
    fn parse_unit_interval_rejects_nan() {
        assert!(parse_unit_interval("NaN").is_err());
    }

    #[test]
    fn cli_accepts_glitch_in_unit_interval() {
        let cli = Cli::try_parse_from(["matrix", "--glitch", "0.0"]).unwrap();
        assert_eq!(cli.glitch, 0.0);
        let cli = Cli::try_parse_from(["matrix", "--glitch", "1.0"]).unwrap();
        assert_eq!(cli.glitch, 1.0);
        let cli = Cli::try_parse_from(["matrix", "--glitch", "0.5"]).unwrap();
        assert_eq!(cli.glitch, 0.5);
    }

    #[test]
    fn cli_rejects_glitch_outside_unit_interval() {
        assert!(Cli::try_parse_from(["matrix", "--glitch", "1.1"]).is_err());
        assert!(Cli::try_parse_from(["matrix", "--glitch", "-0.1"]).is_err());
    }

    #[test]
    fn cli_default_glitch_is_zero() {
        let cli = Cli::try_parse_from(["matrix"]).unwrap();
        assert_eq!(cli.glitch, 0.0);
    }

    #[test]
    fn cli_accepts_mutation_rate_in_unit_interval() {
        let cli = Cli::try_parse_from(["matrix", "--mutation-rate", "0.0"]).unwrap();
        assert_eq!(cli.mutation_rate, 0.0);
        let cli = Cli::try_parse_from(["matrix", "--mutation-rate", "1.0"]).unwrap();
        assert_eq!(cli.mutation_rate, 1.0);
    }

    #[test]
    fn cli_default_mutation_rate_matches_lib_default() {
        let cli = Cli::try_parse_from(["matrix"]).unwrap();
        assert_eq!(cli.mutation_rate, 0.05);
    }

    #[test]
    fn build_config_propagates_glitch_and_mutation() {
        let args = Cli::try_parse_from([
            "matrix",
            "--glitch",
            "0.2",
            "--mutation-rate",
            "0.7",
        ])
        .unwrap();
        let cfg = build_config(&args).unwrap();
        assert_eq!(cfg.glitch, 0.2);
        assert_eq!(cfg.mutation_rate, 0.7);
    }

    struct TempFile(PathBuf);

    impl TempFile {
        fn new(name: &str, contents: &[u8]) -> Self {
            let mut path = std::env::temp_dir();
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            path.push(format!(
                "matrix-test-{}-{}-{}",
                std::process::id(),
                name,
                nanos
            ));
            std::fs::write(&path, contents).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn parse_charset_arg_returns_builtin_for_each_name() {
        for (name, expected) in [
            ("matrix", CharsetSource::Matrix),
            ("ascii", CharsetSource::Ascii),
            ("hex", CharsetSource::Hex),
            ("binary", CharsetSource::Binary),
        ] {
            let got = parse_charset_arg(name).unwrap();
            assert!(matches!(
                (&got, &expected),
                (CharsetSource::Matrix, CharsetSource::Matrix)
                    | (CharsetSource::Ascii, CharsetSource::Ascii)
                    | (CharsetSource::Hex, CharsetSource::Hex)
                    | (CharsetSource::Binary, CharsetSource::Binary)
            ));
        }
    }

    #[test]
    fn parse_charset_arg_rejects_nonexistent_path() {
        let err = parse_charset_arg("/this/path/does/not/exist/charset.txt").unwrap_err();
        assert!(err.contains("neither a built-in"));
    }

    #[test]
    fn parse_charset_arg_accepts_existing_path() {
        let tf = TempFile::new("existing", b"abc");
        let got = parse_charset_arg(tf.path().to_str().unwrap()).unwrap();
        assert!(matches!(got, CharsetSource::Path(_)));
    }

    #[test]
    fn load_charset_filters_whitespace_and_controls() {
        let tf = TempFile::new("filter", b"abc\ndef\tghi  xyz");
        let chars = load_charset_from_path(tf.path()).unwrap();
        for c in &chars {
            assert!(!c.is_whitespace() && !c.is_control());
        }
        assert!(chars.contains(&'a'));
        assert!(chars.contains(&'z'));
    }

    #[test]
    fn load_charset_dedupes() {
        let tf = TempFile::new("dedupe", b"aabbccaa");
        let chars = load_charset_from_path(tf.path()).unwrap();
        assert_eq!(chars, vec!['a', 'b', 'c']);
    }

    #[test]
    fn load_charset_rejects_empty_after_filtering() {
        let tf = TempFile::new("empty", b"   \n\n\t\t");
        let err = load_charset_from_path(tf.path()).unwrap_err();
        assert!(format!("{err:#}").contains("no usable characters"));
    }

    #[test]
    fn load_charset_rejects_too_large_file() {
        let big = vec![b'a'; (MAX_CHARSET_FILE_BYTES + 1) as usize];
        let tf = TempFile::new("big", &big);
        let err = load_charset_from_path(tf.path()).unwrap_err();
        assert!(format!("{err:#}").contains("maximum"));
    }

    #[test]
    fn load_charset_rejects_non_utf8() {
        let tf = TempFile::new("nonutf8", &[0xFF, 0xFE, 0xFD]);
        let err = load_charset_from_path(tf.path()).unwrap_err();
        assert!(format!("{err:#}").contains("UTF-8"));
    }
}
