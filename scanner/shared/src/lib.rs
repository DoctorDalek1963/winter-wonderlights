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

use serde::{Deserialize, Serialize};

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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs, reason = "the variants are just compass directions")]
pub enum CompassDirection {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
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
        /// The unique ID of this request. Used to make sure the server and client are in sync.
        id: u32,
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
        /// The unique ID of this request. Used to make sure the server and client are in sync.
        id: u32,

        /// The position of the brightest pixel in the image as `(x, y)` with (0, 0) in the bottom
        /// left.
        brightest_pixel_pos: (u32, u32),
    },
}

/// A message from the server to the controller client.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerToControllerMsg {
    /// A generic message from the server to a client.
    Generic(GenericServerToClientMsg),

    /// The camera has finished taking photos and the tree should be rotated to the next angle.
    PhotoSequenceDone,
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
    },
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
