//! This module provides the [`AiSnake`] effect.

#[cfg(feature = "config-impls")]
pub use config::AiSnakeConfig;

#[cfg(feature = "effect-impls")]
pub use effect::AiSnake;

use crate::effects::prelude::*;

/// Contains the config for the [`AiSnake`] effect.
#[cfg(feature = "config-impls")]
mod config {
    use super::*;

    /// The config for the [`AiSnake`] effect.
    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, BaseEffectConfig)]
    pub struct AiSnakeConfig {
        /// How many milliseconds we wait between rendering steps.
        pub milliseconds_per_step: u64,

        /// The number of evenly spaced lattice points across the diameter of the bottom of the tree.
        pub lattice_points_across_diameter: u8,

        /// Should we allow the snake to move diagonally?
        pub allow_diagonal_movement: bool,

        /// The colour of the head.
        pub head_colour: RGBArray,

        /// The colour of the tail.
        pub tail_colour: RGBArray,

        /// The colour of the apple.
        pub apple_colour: RGBArray,
    }

    impl Default for AiSnakeConfig {
        fn default() -> Self {
            Self {
                milliseconds_per_step: 1500,
                lattice_points_across_diameter: 6,
                allow_diagonal_movement: false,
                head_colour: [14, 252, 10],
                tail_colour: [2, 140, 0],
                apple_colour: [252, 20, 20],
            }
        }
    }

    impl EffectConfig for AiSnakeConfig {
        fn render_options_gui(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) -> bool {
            let mut config_changed = false;

            config_changed |= ui
                .add(
                    egui::Slider::new(&mut self.milliseconds_per_step, 0..=10_000)
                        .text("Milliseconds per step")
                        .suffix("ms"),
                )
                .changed();

            config_changed |= ui
                .add(
                    egui::Slider::new(&mut self.lattice_points_across_diameter, 0..=60)
                        .text("Lattice points across diameter"),
                )
                .changed();

            config_changed |= ui
                .checkbox(
                    &mut self.allow_diagonal_movement,
                    "Allow diagonal movement?",
                )
                .changed();

            ui.add_space(UI_SPACING);

            config_changed |= colour_picker(ui, &mut self.head_colour, "Head colour").changed();
            config_changed |= colour_picker(ui, &mut self.tail_colour, "Tail colour").changed();
            config_changed |= colour_picker(ui, &mut self.apple_colour, "Apple colour").changed();

            config_changed
        }
    }
}

/// Contains the [`AiSnake`] effect itself.
#[cfg(feature = "effect-impls")]
mod effect {
    use super::*;
    use rand::Rng;
    use std::collections::VecDeque;
    use ww_gift_coords::{GIFTCoords, COORDS};

    /// A coordinate in snake space.
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
    struct Coord(i8, i8, i8);

    impl Coord {
        /// Check if this coordinate is adjacent to another one.
        fn is_adjacent(self, other: Self) -> bool {
            let diffs = (
                self.0.abs_diff(other.0),
                self.1.abs_diff(other.1),
                self.2.abs_diff(other.2),
            );
            diffs == (1, 0, 0) || diffs == (0, 1, 0) || diffs == (0, 0, 1)
        }

        /// Convert the coordinate to a GIFT coordinate.
        fn to_gift(self, cell_width: f32) -> (f32, f32, f32) {
            (
                self.0 as f32 * cell_width,
                self.1 as f32 * cell_width,
                self.2 as f32 * cell_width,
            )
        }
    }

    /// A snake that can move through snake space.
    #[derive(Clone, Debug, PartialEq)]
    struct Snake {
        /// The lattice of points that the snake can move through.
        lattice: Lattice,

        /// The head of the snake.
        head: Coord,

        /// The tail components of the snake.
        tail: VecDeque<Coord>,
    }

    impl Snake {
        /// Create a new snake in the given lattice, placing the head at a random point.
        fn new(lattice: Lattice, rng: &mut StdRng) -> Self {
            let head = lattice
                .random_point(rng)
                .expect_or_log("Should have at least one point in lattice");

            Self {
                lattice,
                head,
                tail: VecDeque::new(),
            }
        }

        /// Move the head to the new position.
        fn move_head(&mut self, new_pos: Coord) {
            debug_assert!(
                self.head.is_adjacent(new_pos),
                "Can only move head to adjacent position"
            );

            self.tail.pop_back();

            debug_assert!(
                !self.tail.contains(&new_pos),
                "tail must not contain new_pos"
            );

            self.tail.push_front(self.head);
            self.head = new_pos;
        }

        /// View the underlying lattice of points.
        fn lattice(&self) -> &Lattice {
            &self.lattice
        }
    }

    /// A collection of valid lattice points.
    #[derive(Clone, Debug, PartialEq)]
    struct Lattice {
        /// The points in the lattice.
        points: Box<[Coord]>,

        /// The width of each cell in GIFT coord units.
        cell_width: f32,
    }

    impl Lattice {
        /// Create a new lattice from the given coordinates and settings.
        #[instrument(skip_all)]
        fn new(coords: &GIFTCoords, config: &AiSnakeConfig) -> Self {
            let bound = (config.lattice_points_across_diameter / 2) as i8;
            let range = -bound..=bound;
            let square: Box<[_]> = range
                .clone()
                .flat_map(|x| range.clone().map(move |y| (x, y)))
                .collect();

            let max_vertical: i8 = (coords.max_z()
                / (2.0 / (config.lattice_points_across_diameter as f32)))
                .floor() as i8;
            let z_range = 0..=max_vertical;

            let cell_width = 2.0 / config.lattice_points_across_diameter as f32;

            let points: Box<[Coord]> = z_range
                .into_iter()
                .flat_map(|z| square.iter().map(move |(x, y)| Coord(*x, *y, z)))
                .filter(|coord| {
                    let (px, py, pz) = coord.to_gift(cell_width);
                    coords.coords().iter().any(|&(lx, ly, lz)| {
                        let dx = px - lx;
                        let dy = py - ly;
                        let dz = pz - lz;

                        #[allow(
                            clippy::suboptimal_flops,
                            reason = "This format offers better clarity"
                        )]
                        let dist = f32::sqrt(dx * dx + dy * dy + dz * dz);

                        dist <= cell_width / 2.0
                    })
                })
                .collect();

            info!(num_points = points.len(), "Created lattice for AI snake");

            Self {
                points,
                cell_width: 2.0 / config.lattice_points_across_diameter as f32,
            }
        }

        /// Iterate over the points in the lattice as GIFT coordinates.
        fn iter_gift_coords(&self) -> impl Iterator<Item = (f32, f32, f32)> + '_ {
            self.points
                .iter()
                .map(|coord| coord.to_gift(self.cell_width))
        }

        /// Get a random point from the lattice.
        fn random_point(&self, rng: &mut StdRng) -> Option<Coord> {
            let idx = rng.gen_range(0..self.points.len());
            self.points.get(idx).copied()
        }
    }

    /// Light up each light individually, one-by-one.
    #[derive(Clone, Debug, PartialEq, BaseEffect)]
    pub struct AiSnake {
        /// The config for this effect.
        config: AiSnakeConfig,

        /// The RNG to use for randomness.
        rng: StdRng,

        /// The snake itself.
        snake: Snake,
    }

    impl Effect for AiSnake {
        fn from_config(config: AiSnakeConfig) -> Self {
            let mut rng = rng!();
            let snake = Snake::new(Lattice::new(&COORDS, &config), &mut rng);

            Self { config, rng, snake }
        }

        #[allow(
            clippy::semicolon_if_nothing_returned,
            reason = "this is a bodge for #[end_loop_in_test_or_bench]"
        )]
        async fn run(self, driver: &mut dyn Driver) {
            let radius = 1.2 / self.config.lattice_points_across_diameter as f32;

            #[end_loop_in_test_or_bench]
            loop {
                for point in self.snake.lattice.iter_gift_coords() {
                    driver.display_frame(FrameType::Frame3D(Frame3D::new(
                        vec![FrameObject {
                            object: Object::Sphere {
                                center: point.into(),
                                radius,
                            },
                            colour: self.config.head_colour,
                            fadeoff: 0.05,
                        }],
                        false,
                    )));

                    sleep!(Duration::from_millis(self.config.milliseconds_per_step));
                    driver.clear();
                }
            }
        }
    }
}
