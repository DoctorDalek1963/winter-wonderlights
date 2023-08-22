//! This module contains effects that demonstrate interesting computation, such as pathfinding.

pub mod ai_snake;

#[cfg(feature = "effect-impls")]
pub use self::ai_snake::AiSnake;

#[cfg(feature = "config-impls")]
pub use self::ai_snake::AiSnakeConfig;
