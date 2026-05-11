//! Frame-rendering benchmarks. Spec §9 target: < 1 ms per frame on a 200×60
//! terminal on a modern CPU.
//!
//! Run with: `cargo bench --bench render`

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidget;

use matrix_rain::{MatrixConfig, MatrixRain, MatrixRainState};

const SEED: u64 = 0xC0FFEE;
const WARMUP_FRAMES: usize = 60;

/// Drive enough renders to fill the column field before measurement, so the
/// bench measures steady-state cost rather than cold-start allocation.
fn warm(area: Rect, cfg: &MatrixConfig, state: &mut MatrixRainState, buf: &mut Buffer) {
    for _ in 0..WARMUP_FRAMES {
        MatrixRain::new(cfg).render(area, buf, state);
        state.tick();
    }
}

fn bench_default_200x60(c: &mut Criterion) {
    let area = Rect::new(0, 0, 200, 60);
    let cfg = MatrixConfig::default();
    let mut state = MatrixRainState::with_seed(SEED);
    let mut buf = Buffer::empty(area);
    warm(area, &cfg, &mut state, &mut buf);

    c.bench_function("render_200x60_default", |b| {
        b.iter(|| {
            MatrixRain::new(&cfg).render(black_box(area), &mut buf, &mut state);
        });
    });
}

fn bench_per_tier_200x60(c: &mut Criterion) {
    let area = Rect::new(0, 0, 200, 60);
    let cfg = MatrixConfig::default();
    let mut group = c.benchmark_group("render_200x60_by_tier");
    // 200 × 60 = 12000 cells per frame; report per-cell amortized cost too.
    group.throughput(Throughput::Elements((area.width as u64) * (area.height as u64)));

    for &tier in &[16u16, 256u16, u16::MAX] {
        let label = match tier {
            16 => "16-color",
            256 => "256-color",
            _ => "truecolor",
        };
        let mut state = MatrixRainState::with_seed(SEED);
        state.set_color_count(tier);
        let mut buf = Buffer::empty(area);
        warm(area, &cfg, &mut state, &mut buf);

        group.bench_with_input(BenchmarkId::from_parameter(label), &tier, |b, _| {
            b.iter(|| {
                MatrixRain::new(&cfg).render(black_box(area), &mut buf, &mut state);
            });
        });
    }
    group.finish();
}

fn bench_size_scaling(c: &mut Criterion) {
    let cfg = MatrixConfig::default();
    let mut group = c.benchmark_group("render_size_scaling");

    for &(w, h) in &[(80u16, 24u16), (120, 40), (200, 60), (300, 80)] {
        let area = Rect::new(0, 0, w, h);
        let mut state = MatrixRainState::with_seed(SEED);
        let mut buf = Buffer::empty(area);
        warm(area, &cfg, &mut state, &mut buf);

        group.throughput(Throughput::Elements((w as u64) * (h as u64)));
        group.bench_with_input(
            BenchmarkId::new("render", format!("{}x{}", w, h)),
            &area,
            |b, &a| {
                b.iter(|| {
                    MatrixRain::new(&cfg).render(black_box(a), &mut buf, &mut state);
                });
            },
        );
    }
    group.finish();
}

fn bench_with_glitch_and_mutation(c: &mut Criterion) {
    let area = Rect::new(0, 0, 200, 60);
    let cfg = MatrixConfig::builder()
        .mutation_rate(0.2)
        .glitch(0.1)
        .build()
        .unwrap();
    let mut state = MatrixRainState::with_seed(SEED);
    let mut buf = Buffer::empty(area);
    warm(area, &cfg, &mut state, &mut buf);

    c.bench_function("render_200x60_mutation_glitch", |b| {
        b.iter(|| {
            MatrixRain::new(&cfg).render(black_box(area), &mut buf, &mut state);
        });
    });
}

criterion_group!(
    benches,
    bench_default_200x60,
    bench_per_tier_200x60,
    bench_size_scaling,
    bench_with_glitch_and_mutation,
);
criterion_main!(benches);
