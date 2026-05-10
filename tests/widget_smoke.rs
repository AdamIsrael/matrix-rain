use ratatui::backend::TestBackend;
use ratatui::Terminal;

use matrix_rain::{MatrixConfig, MatrixRain, MatrixRainState};

#[test]
fn renders_1000_frames_without_panic_preserving_per_column_invariant() {
    let cfg = MatrixConfig::default();
    let mut state = MatrixRainState::with_seed(0xDEADBEEF);
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();

    for _ in 0..1000 {
        terminal
            .draw(|f| {
                f.render_stateful_widget(MatrixRain::new(&cfg), f.size(), &mut state);
            })
            .unwrap();
        assert_eq!(state.streams_len(), 80);
        state.tick();
    }
}

#[test]
fn resize_cycles_keep_streams_per_column() {
    let cfg = MatrixConfig::default();
    let mut state = MatrixRainState::with_seed(0xC0FFEE);

    // Grow → shrink → grow → resize-up → shrink-vertical → grow back. 50 frames each.
    let cycles: &[(u16, u16)] = &[
        (40, 24),
        (80, 24),
        (20, 24),
        (100, 30),
        (50, 10),
        (80, 24),
    ];
    for &(w, h) in cycles {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        for _ in 0..50 {
            terminal
                .draw(|f| {
                    f.render_stateful_widget(MatrixRain::new(&cfg), f.size(), &mut state);
                })
                .unwrap();
            assert_eq!(state.streams_len(), w as usize);
            state.tick();
        }
    }
    assert_eq!(state.streams_len(), 80);
}

#[test]
fn oscillating_resize_within_one_terminal_does_not_panic() {
    let cfg = MatrixConfig::default();
    let mut state = MatrixRainState::with_seed(0xBADF00D);
    let mut terminal = Terminal::new(TestBackend::new(50, 20)).unwrap();

    let alternating = [(50u16, 20u16), (10, 5), (50, 20), (10, 5), (50, 20)];
    for &(w, h) in alternating.iter().cycle().take(50) {
        terminal.backend_mut().resize(w, h);
        terminal
            .draw(|f| {
                f.render_stateful_widget(MatrixRain::new(&cfg), f.size(), &mut state);
            })
            .unwrap();
        assert_eq!(state.streams_len(), w as usize);
    }
}

#[test]
fn empty_then_non_empty_recovers_via_first_render_path() {
    let cfg = MatrixConfig::default();
    let mut state = MatrixRainState::with_seed(0xFEEDFACE);
    let mut terminal = Terminal::new(TestBackend::new(20, 10)).unwrap();

    terminal
        .draw(|f| {
            f.render_stateful_widget(MatrixRain::new(&cfg), f.size(), &mut state);
        })
        .unwrap();
    assert_eq!(state.streams_len(), 20);

    terminal.backend_mut().resize(0, 10);
    terminal
        .draw(|f| {
            f.render_stateful_widget(MatrixRain::new(&cfg), f.size(), &mut state);
        })
        .unwrap();
    assert_eq!(state.streams_len(), 0);

    terminal.backend_mut().resize(15, 10);
    terminal
        .draw(|f| {
            f.render_stateful_widget(MatrixRain::new(&cfg), f.size(), &mut state);
        })
        .unwrap();
    assert_eq!(state.streams_len(), 15);
}
