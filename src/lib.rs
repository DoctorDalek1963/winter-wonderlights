//! This crate is designed to display 3D effects on a Christmas tree in real time.
//!
//! The effects are all contained in the [`effects`] module.
//!
//! The drivers are all contained in the [`drivers`] module. These include a driver to debug effects
//! and just write all the data to a log with tracing, as well as a driver to display the effects
//! on a virtual tree, and a driver for real lights. All drivers except the debug one are locked
//! behind crate features.
//!
//! For effect math and tree coordinates, the vertical axis is z. Bevy uses y as the vertical axis,
//! but the conversion is handled in `drivers::virtual_tree`.

#![feature(is_some_and)]
#![feature(never_type)]
#![feature(stmt_expr_attributes)]

pub mod drivers {
    pub use ww_drivers::*;
}
pub mod effects {
    pub use ww_effects::*;
}
pub mod frame {
    pub use ww_frame::*;
}
pub mod gift_coords {
    pub use ww_gift_coords::*;
}
