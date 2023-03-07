//! This crate is designed to display 3D effects on a Christmas tree in real time. All
//! functionality is split into other crates; this one just collects them into one library.
//!
//! For effect maths and tree coordinates, the vertical axis is z.

#![feature(is_some_and)]
#![feature(never_type)]
#![feature(stmt_expr_attributes)]

pub mod drivers {
    pub use ww_driver_impl::*;
    pub use ww_driver_trait::*;
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
