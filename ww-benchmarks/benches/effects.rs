use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use strum::IntoEnumIterator;
use ww_driver_trait::Driver;
use ww_effects::EffectNameList;
use ww_frame::{FrameType, RGBArray};
use ww_gift_coords::COORDS;

struct SimpleDriver;

impl Driver for SimpleDriver {
    unsafe fn init() -> Self {
        Self
    }

    fn display_frame(&mut self, frame: FrameType) {
        // Do nothing, but don't optimise this away
        black_box(frame);
    }
}

struct ConvertFrameDriver {
    current_frame: Vec<RGBArray>,
}

impl Driver for ConvertFrameDriver {
    unsafe fn init() -> Self {
        Self {
            current_frame: Vec::with_capacity(COORDS.lights_num()),
        }
    }

    fn display_frame(&mut self, frame: FrameType) {
        match frame {
            FrameType::Off => self.current_frame.fill([0; 3]),
            FrameType::RawData(data) => self.current_frame = data,
            FrameType::Frame3D(frame) => self.current_frame = frame.to_raw_data(),
        };
    }
}

fn debug_effects(c: &mut Criterion) {
    for name in EffectNameList::iter() {
        c.bench_function(&format!("(SimpleDriver) {}", name.effect_name()), |b| {
            let mut effect = name.default_dispatch();
            let config = name.config_name().default_dispatch();
            let mut driver = unsafe { SimpleDriver::init() };

            b.iter(|| {
                if let Some(number) = effect.loops_to_test() {
                    for i in 0..u16::from(number) {
                        if let Some((frame, _duration)) = effect.next_frame(&config) {
                            driver.display_frame(frame);
                        } else {
                            panic!(
                                "Effect {} said it would loop {number} times but terminated after only {i} loops",
                                effect.effect_name()
                            );
                        }
                    }
                } else {
                    while let Some((frame, _duration)) = effect.next_frame(&config) {
                        driver.display_frame(frame);
                    }
                }
            });
        });
        c.bench_function(
            &format!("(ConvertFrameDriver) {}", name.effect_name()),
            |b| {
                let mut effect = name.default_dispatch();
                let config = name.config_name().default_dispatch();
                let mut driver = unsafe { ConvertFrameDriver::init() };

                b.iter(|| {
                    if let Some(number) = effect.loops_to_test() {
                        for i in 0..u16::from(number) {
                            if let Some((frame, _duration)) = effect.next_frame(&config) {
                                driver.display_frame(frame);
                            } else {
                                panic!(
                                    "Effect {} said it would loop {number} times but terminated after only {i} loops",
                                    effect.effect_name()
                                );
                            }
                        }
                    } else {
                        while let Some((frame, _duration)) = effect.next_frame(&config) {
                            driver.display_frame(frame);
                        }
                    }
                });
            },
        );
    }
}

criterion_group! { effects, debug_effects }
criterion_main! { effects }
