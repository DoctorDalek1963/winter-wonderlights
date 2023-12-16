//! This crate handles messages sent between the scanner's server and the clients.
//!
//! There are two types of client and the server has to be connected to both to function properly.
//! A *camera* client has a camera and take the pictures of the tree. A *controller* client just
//! tells the server when everything is set up to start taking pictures, and what angle the camera
//! is at from the tree.
//!
//! When a client tries to connect to the server, it *must* first send a [`ClientType`] declaration
//! and then the appropriate `EstablishConnection`(s). A [`ClientType`] declaration is a `[u8; 4]`,
//! where the first three bytes are from [`DECLARE_CLIENT_TYPE_MAGIC`] and the last byte is from
//! [`ClientType`].

#![feature(lint_reasons)]

use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use strum::EnumIter;

/// The magic numbers expected at the start of a [`ClientType`] declaration see [the module
/// documentation](self) for details.
pub const DECLARE_CLIENT_TYPE_MAGIC: [u8; 3] = [0xBE, 0xEF, 0xAF];

/// An RGB colour.
pub type RGBArray = [u8; 3];

/// The type of client.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ClientType {
    /// This client operates the camera.
    Camera,

    /// This client tells the server when the user is ready to take pictures and which side of the
    /// tree is facing the camera.
    Controller,
}

impl TryFrom<u8> for ClientType {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let camera = Self::Camera as u8;
        let controller = Self::Controller as u8;

        if value == camera {
            Ok(Self::Camera)
        } else if value == controller {
            Ok(Self::Controller)
        } else {
            Err("Failed to convert u8 to ClientType")
        }
    }
}

/// Directions of a compass.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter)]
#[allow(missing_docs, reason = "the variants are just compass directions")]
#[repr(u8)]
pub enum CompassDirection {
    North = 1,
    NorthEast = 2,
    East = 4,
    SouthEast = 8,
    South = 16,
    SouthWest = 32,
    West = 64,
    NorthWest = 128,
}

impl CompassDirection {
    pub fn name(&self) -> &'static str {
        match self {
            Self::North => "North",
            Self::NorthEast => "North East",
            Self::East => "East",
            Self::SouthEast => "South East",
            Self::South => "South",
            Self::SouthWest => "South West",
            Self::West => "West",
            Self::NorthWest => "North West",
        }
    }

    /// How many clockwise turns of 45 degrees do we have to do to get here from [`Self::North`]?
    pub fn turns_from_north(&self) -> u8 {
        match self {
            Self::North => 0,
            Self::NorthEast => 1,
            Self::East => 2,
            Self::SouthEast => 3,
            Self::South => 4,
            Self::SouthWest => 5,
            Self::West => 6,
            Self::NorthWest => 7,
        }
    }
}

/// A flag of completed [`CompassDirection`]s.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct CompassDirectionFlags(u8);

bitflags! {
    impl CompassDirectionFlags: u8 {
        const North     = 0b0000_0001;
        const NorthEast = 0b0000_0010;
        const East      = 0b0000_0100;
        const SouthEast = 0b0000_1000;
        const South     = 0b0001_0000;
        const SouthWest = 0b0010_0000;
        const West      = 0b0100_0000;
        const NorthWest = 0b1000_0000;
    }
}

impl CompassDirectionFlags {
    /// Having done these directions, are we ready to finish scanning?
    pub fn is_ready_to_finish(&self) -> bool {
        self.contains(Self::North | Self::East | Self::South | Self::West)
    }
}

impl From<CompassDirection> for CompassDirectionFlags {
    fn from(value: CompassDirection) -> Self {
        match value {
            CompassDirection::North => Self::North,
            CompassDirection::NorthEast => Self::NorthEast,
            CompassDirection::East => Self::East,
            CompassDirection::SouthEast => Self::SouthEast,
            CompassDirection::South => Self::South,
            CompassDirection::SouthWest => Self::SouthWest,
            CompassDirection::West => Self::West,
            CompassDirection::NorthWest => Self::NorthWest,
        }
    }
}

/// A generic message from the server to a client.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenericServerToClientMsg {
    /// Accept an incoming connection request from the client.
    AcceptConnection,

    /// Reject an incoming connection request from the client.
    RejectConnection,

    /// Terminate the current connection.
    TerminateConnection,

    /// The server is ready to start.
    ServerReady,

    /// The server is not ready to start.
    ServerNotReady,
}

/// A message from the server to the camera client.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(
    variant_size_differences,
    reason = "every variant is small enough to not be an issue"
)]
pub enum ServerToCameraMsg {
    /// A generic message from the server to a client.
    Generic(GenericServerToClientMsg),

    /// Tell the client that the lights are ready and a photo should be taken.
    TakePhoto {
        /// The index of the light being photographed.
        light_idx: u32,
    },
}

/// Information about the camera.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BasicCameraInfo {
    /// The total resolution of the camera as `(width, height)`.
    pub resolution: (u32, u32),
}

/// A message from the camera client to the server.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CameraToServerMsg {
    /// Declare the type of this client. See [the module documentation](self) for details.
    DeclareClientType,

    /// Try to establish a connection with the server.
    EstablishConnection(BasicCameraInfo),

    /// Tell the server that a photo has been taken and send the position of the brightest pixel.
    PhotoTaken {
        /// The index of the light being photographed.
        light_idx: u32,

        /// The position of the brightest pixel in the image as `(x, y)` with (0, 0) in the top
        /// left.
        brightest_pixel_pos: (u32, u32),

        /// The brightness of the brightest pixel in the image. Uses the full `u8` range.
        pixel_brightness: u8,
    },
}

/// A message from the server to the controller client.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerToControllerMsg {
    /// A generic message from the server to a client.
    Generic(GenericServerToClientMsg),

    /// The camera has finished taking photos and the tree should be rotated to the next angle.
    PhotoSequenceDone {
        /// The sides of the tree that we've successfully finished scanning.
        finished_sides: CompassDirectionFlags,
    },

    /// The photo sequence was cancelled.
    PhotoSequenceCancelled {
        /// The sides of the tree that we've successfully finished scanning.
        finished_sides: CompassDirectionFlags,
    },

    /// The progress that the server has made scanning the lights from the current direction.
    ProgressUpdate {
        /// The number of lights that have already been scanned.
        scanned: u16,

        /// The total number of lights to scan.
        total: u16,
    },
}

/// A message from the controller client to the server.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControllerToServerMsg {
    /// Declare the type of this client. See [the module documentation](self) for details.
    DeclareClientType,

    /// Try to establish a connection with the server.
    EstablishConnection,

    /// Everything is set up and ready for the camera to take photos.
    ReadyToTakePhotos {
        /// Which side of the tree is facing the camera?
        camera_alignment: CompassDirection,

        /// The time to pause between taking photos, in milliseconds.
        pause_time_ms: u16,
    },

    /// Tell the server to stop taking photos.
    CancelPhotoSequence,

    /// We're finished scanning the tree, and the server should process the photos to make a GIFT
    /// file.
    FinishScanning,
}

/// This module just contains the [`ClientToServerMsg`](client_impl::ClientToServerMsg) trait.
#[cfg(feature = "client-impl")]
pub mod client_impl {
    use super::{CameraToServerMsg, ClientType, ControllerToServerMsg, DECLARE_CLIENT_TYPE_MAGIC};

    /// This module contains a simple [`Sealed`](self::private::Sealed) trait to prevent
    /// [`ClientToServerMsg`] being implemented on foreign types.
    mod private {
        use super::{CameraToServerMsg, ControllerToServerMsg};

        /// This trait restricts implementors of [`ClientToServerMsg`](super::ClientToServerMsg).
        pub trait Sealed {}

        impl Sealed for CameraToServerMsg {}
        impl Sealed for ControllerToServerMsg {}
    }

    /// A trait that's implemented on both [`CameraToServerMsg`] and [`ControllerToServerMsg`] and
    /// allows the
    /// [`GenericClientWidget`](../../ww_scanner_client/generic_client/struct.GenericClientWidget.html)
    /// to have its
    /// [`send_establish_connection`](../../ww_scanner_client/generic_client/struct.GenericClientWidget.html#method.send_establish_connection)
    /// method.
    pub trait ClientToServerMsg: private::Sealed {
        /// Make a `DeclareClientType` message.
        fn make_declare_client_type_message() -> Self;

        /// If this message is a `DeclareClientType`, return the relevant
        /// `[u8; 4]`, else [`None`].
        fn is_declare_client_type_message(&self) -> Option<[u8; 4]>;
    }

    impl ClientToServerMsg for CameraToServerMsg {
        fn make_declare_client_type_message() -> Self {
            Self::DeclareClientType
        }

        fn is_declare_client_type_message(&self) -> Option<[u8; 4]> {
            if self == &Self::DeclareClientType {
                let [a, b, c] = DECLARE_CLIENT_TYPE_MAGIC;
                Some([a, b, c, ClientType::Camera as u8])
            } else {
                None
            }
        }
    }

    impl ClientToServerMsg for ControllerToServerMsg {
        fn make_declare_client_type_message() -> Self {
            Self::DeclareClientType
        }

        fn is_declare_client_type_message(&self) -> Option<[u8; 4]> {
            if self == &Self::DeclareClientType {
                let [a, b, c] = DECLARE_CLIENT_TYPE_MAGIC;
                Some([a, b, c, ClientType::Controller as u8])
            } else {
                None
            }
        }
    }
}
