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
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, BaseEffectConfig)]
    pub struct AiSnakeConfig {
        /// How many milliseconds we wait between rendering steps.
        pub milliseconds_per_step: u64,

        /// The number of evenly spaced lattice points across the diameter of the bottom of the tree.
        pub lattice_points_across_diameter: u8,

        /// The thickness of the snake and the apple.
        pub thickness: f32,

        /// The fadeoff of the objects in the frame.
        pub fadeoff: f32,

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
                milliseconds_per_step: 250,
                lattice_points_across_diameter: 8,
                thickness: 0.2,
                fadeoff: 0.2,
                allow_diagonal_movement: true,
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
                    egui::Slider::new(&mut self.milliseconds_per_step, 0..=5000)
                        .text("Milliseconds per step")
                        .suffix("ms"),
                )
                .changed();

            config_changed |= ui
                .add(
                    egui::Slider::new(&mut self.lattice_points_across_diameter, 0..=10)
                        .text("Lattice points across diameter"),
                )
                .changed();

            config_changed |= ui
                .add(egui::Slider::new(&mut self.thickness, 0.0..=0.5).text("Thickness"))
                .changed();

            config_changed |= ui
                .add(egui::Slider::new(&mut self.fadeoff, 0.0..=0.5).text("Fadeoff"))
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
    use ordered_float::NotNan;
    use rand::Rng;
    use std::{collections::VecDeque, fmt, iter};
    use ww_gift_coords::{GIFTCoords, COORDS};

    /// A coordinate in snake space.
    #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
    struct Coord(i8, i8, i8);

    impl Coord {
        /// Check if this coordinate is orthogonally adjacent to another one.
        fn is_orthogonal(self, other: Self) -> bool {
            let diffs = (
                self.0.abs_diff(other.0),
                self.1.abs_diff(other.1),
                self.2.abs_diff(other.2),
            );
            diffs == (1, 0, 0) || diffs == (0, 1, 0) || diffs == (0, 0, 1)
        }

        /// Check if this coordinate is diagonally adjacent to another one.
        fn is_diagonal(self, other: Self) -> bool {
            let diffs = (
                self.0.abs_diff(other.0),
                self.1.abs_diff(other.1),
                self.2.abs_diff(other.2),
            );
            diffs == (1, 1, 0) || diffs == (1, 0, 1) || diffs == (0, 1, 1) || diffs == (1, 1, 1)
        }

        /// Convert the coordinate to a GIFT coordinate.
        fn to_gift(self, cell_width: f32) -> (f32, f32, f32) {
            (
                self.0 as f32 * cell_width,
                self.1 as f32 * cell_width,
                self.2 as f32 * cell_width,
            )
        }

        /// Calculate the Euclidean distance between this coordinate and another one.
        fn distance(self, other: Self) -> NotNan<f32> {
            let (dx, dy, dz) = (
                self.0 as f32 - other.0 as f32,
                self.1 as f32 - other.1 as f32,
                self.2 as f32 - other.2 as f32,
            );
            NotNan::new(f32::sqrt(dx.mul_add(dx, dy.mul_add(dy, dz * dz))))
                .expect_or_log("Euclidean distance of snake points should never be NaN")
        }
    }

    /// A snake that can move through snake space.
    #[derive(Clone, PartialEq)]
    struct Snake {
        /// The lattice of points that the snake can move through.
        lattice: Lattice,

        /// The head of the snake.
        head: Coord,

        /// The tail components of the snake.
        tail: VecDeque<Coord>,

        /// The apple that the snake is moving towards.
        apple: Coord,

        /// The path the snake will take to the apple from its current position.
        current_path: VecDeque<Coord>,
    }

    #[allow(
        clippy::missing_fields_in_debug,
        reason = "The lattice is unneeded noise"
    )]
    impl fmt::Debug for Snake {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Snake")
                .field("head", &self.head)
                .field("tail", &self.tail)
                .field("apple", &self.apple)
                .field("current_path", &self.current_path)
                .finish()
        }
    }

    /// An error that can occur from the snake.
    enum SnakeError {
        /// We couldn't find a path from the head to the apple.
        PathfindingFail,

        /// We didn't have enough space to place the apple.
        PlaceAppleFail,
    }

    impl Snake {
        /// Create a new snake in the given lattice, placing the head and apple at a random point,
        /// but not calculating a path.
        fn new(lattice: Lattice, rng: &mut StdRng) -> Self {
            let head = lattice
                .random_point(rng)
                .expect_or_log("Should have at least one point in lattice");

            let mut apple = head;
            while apple == head {
                apple = lattice
                    .random_point(rng)
                    .expect_or_log("Should have at least one point in lattice");
            }

            Self {
                lattice,
                head,
                tail: VecDeque::new(),
                apple,
                current_path: VecDeque::new(),
            }
        }

        /// Clear the tail, and reset the head and apple to random positions.
        fn reset(&mut self, rng: &mut StdRng) {
            self.tail.clear();
            self.current_path.clear();

            self.head = self
                .lattice
                .random_point(rng)
                .expect_or_log("Should have at least one point in lattice");

            self.apple = self.head;
            while self.apple == self.head {
                self.apple = self
                    .lattice
                    .random_point(rng)
                    .expect_or_log("Should have at least one point in lattice");
            }
        }

        /// Move the head to the new position.
        fn move_head(&mut self, new_pos: Coord) {
            debug_assert!(
                self.head.is_orthogonal(new_pos) || self.head.is_diagonal(new_pos),
                "Can only move head to adjacent position"
            );

            if new_pos != self.apple {
                self.tail.pop_back();
            }

            debug_assert!(
                !self.tail.contains(&new_pos),
                "tail must not contain new_pos"
            );

            self.tail.push_front(self.head);
            self.head = new_pos;
        }

        /// Place the apple randomly in the lattice.
        fn place_random_apple(&mut self, rng: &mut StdRng) -> Result<(), SnakeError> {
            let mut apple = self.head;

            // If the snake and apple make up more than 3/4 of the lattice, then tell
            // [`self.advance`] to fail.
            if self.tail.len() + 2 >= self.lattice.points.len() * 3 / 4 {
                return Err(SnakeError::PlaceAppleFail);
            }

            while apple == self.head || self.tail.contains(&apple) {
                apple = self
                    .lattice
                    .random_point(rng)
                    .expect_or_log("Should have at least one point in lattice");
            }

            self.apple = apple;
            Ok(())
        }

        /// Calculate and store the shortest path from the head to the apple, avoiding the tail.
        fn calculate_shortest_path(&mut self, allow_diagonals: bool) {
            self.tail.make_contiguous();
            self.current_path = self
                .lattice
                .shortest_path(
                    self.head,
                    self.apple,
                    self.tail.as_slices().0,
                    allow_diagonals,
                )
                .map_or_else(VecDeque::new, |v| v.into());
            self.current_path.pop_front();
        }

        /// Advance the snake one space towards the apple.
        fn advance(&mut self, rng: &mut StdRng, allow_diagonals: bool) -> Result<(), SnakeError> {
            if let Some(new_pos) = self.current_path.pop_front() {
                self.move_head(new_pos);
            } else {
                self.place_random_apple(rng)?;
                self.calculate_shortest_path(allow_diagonals);
                if self.current_path.is_empty() {
                    return Err(SnakeError::PathfindingFail);
                }
            }

            Ok(())
        }

        /// Get the GIFT coordinates of the snake, starting at the end and moving through the tail.
        fn get_snake_gift_coords(&self) -> Box<[Vec3]> {
            iter::once(self.head)
                .chain(self.tail.iter().copied())
                .map(|coord| Vec3::from(coord.to_gift(self.lattice.cell_width)))
                .collect()
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

                        let dist = f32::sqrt(dx.mul_add(dx, dy.mul_add(dy, dz * dz)));

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

        /// Get a random point from the lattice.
        fn random_point(&self, rng: &mut StdRng) -> Option<Coord> {
            let idx = rng.gen_range(0..self.points.len());
            self.points.get(idx).copied()
        }

        /// Find the shortest path between two points on the lattice
        fn shortest_path(
            &self,
            start: Coord,
            end: Coord,
            avoid: &[Coord],
            allow_diagonals: bool,
        ) -> Option<Vec<Coord>> {
            debug_assert!(
                self.points.contains(&start),
                "Start point must be on lattice"
            );
            debug_assert!(self.points.contains(&end), "End point must be on lattice");

            let path = pathfinding::directed::dijkstra::dijkstra(
                &start,
                |&Coord(x, y, z)| {
                    [
                        Coord(x + 1, y, z),
                        Coord(x - 1, y, z),
                        Coord(x, y + 1, z),
                        Coord(x, y - 1, z),
                        Coord(x, y, z + 1),
                        Coord(x, y, z - 1),
                    ]
                    .into_iter()
                    .chain(if allow_diagonals {
                        vec![
                            Coord(x + 1, y + 1, z),
                            Coord(x + 1, y, z + 1),
                            Coord(x, y + 1, z + 1),
                            Coord(x + 1, y - 1, z),
                            Coord(x + 1, y, z - 1),
                            Coord(x, y + 1, z - 1),
                            Coord(x - 1, y + 1, z),
                            Coord(x - 1, y, z + 1),
                            Coord(x, y - 1, z + 1),
                            Coord(x - 1, y - 1, z),
                            Coord(x - 1, y, z - 1),
                            Coord(x, y - 1, z - 1),
                            Coord(x + 1, y + 1, z + 1),
                            Coord(x + 1, y + 1, z - 1),
                            Coord(x + 1, y - 1, z + 1),
                            Coord(x + 1, y - 1, z - 1),
                            Coord(x - 1, y + 1, z + 1),
                            Coord(x - 1, y + 1, z - 1),
                            Coord(x - 1, y - 1, z + 1),
                            Coord(x - 1, y - 1, z - 1),
                        ]
                        .into_iter()
                    } else {
                        vec![].into_iter()
                    })
                    .filter(|coord| self.points.contains(coord) && !avoid.contains(coord))
                    .map(move |coord| (coord, Coord(x, y, z).distance(coord)))
                },
                |pos| pos == &end,
            )
            .map(|(path, _dist)| path);
            trace!(?start, ?end, ?path);
            path
        }
    }

    /// Create an AI snake that moves through the tree to collect the apple.
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
        async fn run(mut self, driver: &mut dyn Driver) {
            let mut fail_count: u8 = 0;

            #[end_loop_in_test_or_bench]
            loop {
                /// Reset the snake and `fail_count`, pause, and restart the loop.
                macro_rules! reset_snake {
                    () => {
                        fail_count = 0;
                        self.snake.reset(&mut self.rng);
                        sleep!(Duration::from_millis(self.config.milliseconds_per_step));
                        driver.clear();
                        continue;
                    };
                }

                match self
                    .snake
                    .advance(&mut self.rng, self.config.allow_diagonal_movement)
                {
                    Ok(()) => {
                        fail_count = 0;
                        driver.display_frame(FrameType::Frame3D(Frame3D::new(
                            vec![
                                FrameObject {
                                    object: Object::CatmullRomSpline {
                                        points: self.snake.get_snake_gift_coords(),
                                        threshold: self.config.thickness,
                                        start_colour: self.config.head_colour,
                                        end_colour: self.config.tail_colour,
                                    },
                                    colour: [0, 0, 0],
                                    fadeoff: self.config.fadeoff,
                                },
                                FrameObject {
                                    object: Object::Sphere {
                                        center: self
                                            .snake
                                            .apple
                                            .to_gift(self.snake.lattice.cell_width)
                                            .into(),
                                        radius: self.config.thickness,
                                    },
                                    colour: self.config.apple_colour,
                                    fadeoff: self.config.fadeoff,
                                },
                            ],
                            true,
                        )));
                        sleep!(Duration::from_millis(self.config.milliseconds_per_step));
                    }
                    Err(SnakeError::PathfindingFail) => {
                        fail_count += 1;
                        debug!(?fail_count, "Pathfinding fail");
                        if fail_count == 10 {
                            reset_snake!();
                        }
                    }
                    Err(SnakeError::PlaceAppleFail) => {
                        debug!("Place apple fail");
                        reset_snake!();
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{traits::Effect, TestDriver};

    #[tokio::test]
    async fn ai_snake_test() {
        let mut driver = TestDriver::new(10);
        AiSnake::default().run(&mut driver).await;

        insta::assert_ron_snapshot!(driver.data);
    }
}
