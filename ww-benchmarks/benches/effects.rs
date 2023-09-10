use criterion::{criterion_group, criterion_main, Criterion};
use strum::IntoEnumIterator;
use ww_driver_trait::Driver;
use ww_effects::EffectNameList;
use ww_frame::{FrameType, RGBArray};

const LIGHTS_NUM: usize = 500;

static mut SIMPLE_DRIVER: SimpleDriver = SimpleDriver {};

static mut CONVERT_FRAME_DRIVER: ConvertFrameDriver = ConvertFrameDriver {
    current_frame: vec![],
};

struct SimpleDriver;

impl Driver for SimpleDriver {
    unsafe fn init() -> Self {
        Self
    }

    fn display_frame(&mut self, _frame: FrameType) {}

    fn get_lights_count(&self) -> usize {
        LIGHTS_NUM
    }
}

struct ConvertFrameDriver {
    current_frame: Vec<RGBArray>,
}

impl Driver for ConvertFrameDriver {
    unsafe fn init() -> Self {
        Self {
            current_frame: Vec::with_capacity(LIGHTS_NUM),
        }
    }

    fn display_frame(&mut self, frame: FrameType) {
        match frame {
            FrameType::Off => self.current_frame.fill([0; 3]),
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
        .expect("Should be able to build tokio runtime");

    for name in EffectNameList::iter() {
        c.bench_function(&format!("(SimpleDriver) {}", name.effect_name()), |b| {
            let effect = name.default_dispatch();
            b.to_async(&runtime)
                .iter(|| effect.clone().run(unsafe { &mut SIMPLE_DRIVER }));
        });
        c.bench_function(
            &format!("(ConvertFrameDriver) {}", name.effect_name()),
            |b| {
                let effect = name.default_dispatch();
                b.to_async(&runtime)
                    .iter(|| effect.clone().run(unsafe { &mut CONVERT_FRAME_DRIVER }));
            },
        );
    }
}

criterion_group! { effects, debug_effects }
criterion_main! { effects }
