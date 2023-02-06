#![cfg(feature = "bench")]

use criterion::{criterion_group, criterion_main, Criterion};
use winter_wonderlights::{drivers::Driver, effects::EffectList, frame::FrameType};

const LIGHTS_NUM: usize = 500;

static mut DRIVER: BenchDriver = BenchDriver {};

struct BenchDriver;

impl Driver for BenchDriver {
    fn display_frame(&mut self, _frame: FrameType) {}

    fn get_lights_count(&self) -> usize {
        LIGHTS_NUM
    }
}

fn debug_effects(c: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();

    macro_rules! benchmark_methods {
        ( $( $name:ident ),* ) => {
            $(
                let method = EffectList::$name.create_run_method();
                c.bench_function(stringify!($name), |b| {
                    b.to_async(&runtime).iter(|| method(unsafe { &mut DRIVER }));
                });
            )*
        };
    }

    benchmark_methods!(DebugOneByOne, DebugBinaryIndex);
}

criterion_group!(effects, debug_effects);
criterion_main!(effects);
