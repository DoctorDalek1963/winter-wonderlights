use criterion::{criterion_group, criterion_main, Criterion};
use ww_driver_trait::Driver;
use ww_effects::{effects::*, EffectDispatchList};
use ww_frame::{FrameType, RGBArray};

const LIGHTS_NUM: usize = 500;

static mut SIMPLE_DRIVER: SimpleDriver = SimpleDriver {};

static mut CONVERT_FRAME_DRIVER: ConvertFrameDriver = ConvertFrameDriver {
    current_frame: vec![],
};

struct SimpleDriver;

impl Driver for SimpleDriver {
    fn display_frame(&mut self, _frame: FrameType) {}

    fn get_lights_count(&self) -> usize {
        LIGHTS_NUM
    }
}

struct ConvertFrameDriver {
    current_frame: Vec<RGBArray>,
}

impl Driver for ConvertFrameDriver {
    fn display_frame(&mut self, frame: FrameType) {
        match frame {
            FrameType::Off => self.current_frame = vec![[0, 0, 0]; LIGHTS_NUM],
            FrameType::RawData(data) => self.current_frame = data,
            FrameType::Frame3D(frame) => self.current_frame = frame.to_raw_data(),
        };
    }

    fn get_lights_count(&self) -> usize {
        LIGHTS_NUM
    }
}

fn debug_effects(c: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();

    macro_rules! benchmark_effects_simple {
        ( $( $name:ident ),* ) => {
            $(
                c.bench_function(concat!("(SimpleDriver) ", stringify!($name)), |b| {
                    let effect = EffectDispatchList::$name($name::default());
                    b.to_async(&runtime).iter(|| effect.clone().run(unsafe { &mut SIMPLE_DRIVER }));
                });
            )*
        };
    }

    macro_rules! benchmark_effects_convert_frame {
        ( $( $name:ident ),* ) => {
            $(
                c.bench_function(concat!("(ConvertFrameDriver) ", stringify!($name)), |b| {
                    let effect = EffectDispatchList::$name($name::default());
                    b.to_async(&runtime).iter(|| effect.clone().run(unsafe { &mut CONVERT_FRAME_DRIVER }));
                });
            )*
        };
    }

    macro_rules! benchmark_effects {
        ( $( $name:ident ),* ) => {
            benchmark_effects_simple!( $( $name ),* );
            benchmark_effects_convert_frame!( $( $name ),* );
        };
    }

    benchmark_effects!(DebugOneByOne, DebugBinaryIndex, MovingPlane);
}

criterion_group! { effects, debug_effects }
criterion_main! { effects }
