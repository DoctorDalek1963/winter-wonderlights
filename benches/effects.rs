#![cfg(feature = "bench")]

use criterion::{criterion_group, criterion_main, Criterion};
use winter_wonderlights::{
    drivers::Driver,
    effects::{self, Effect},
    frame::FrameType,
};

const LIGHTS_NUM: usize = 500;

struct BenchDriver;

impl Driver for BenchDriver {
    fn display_frame(&mut self, _frame: FrameType) {}

    fn get_lights_count(&self) -> usize {
        LIGHTS_NUM
    }
}

fn debug_effects(c: &mut Criterion) {
    let mut driver = BenchDriver {};

    c.bench_function("DebugOneByOne", |b| {
        b.iter(|| effects::DebugOneByOne {}.run(&mut driver));
    });

    c.bench_function("DebugBinaryIndex", |b| {
        b.iter(|| effects::DebugBinaryIndex {}.run(&mut driver));
    });
}

criterion_group!(effects, debug_effects);
criterion_main!(effects);
