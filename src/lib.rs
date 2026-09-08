//! Budlum 3.0 seed transfer core: content to compact form to optical frames
//! to recipe, and the exact reverse way back.
//!
//! The pipeline stages (plan CH A1-A5):
//!
//! ```text
//! content bytes
//!   -> payload      (zlib-if-shrinks container + commitment)
//!   -> carousel     (systematic fountain drops + repair)
//!   -> frame        (self-describing optical frames, digest-bound)
//!   -> matrix / png (ISO QR symbols, deterministic raster)
//!   -> video        (raw frame carrier with commitment)
//!   -> recipe       (public or sealed parameters that re-emit the stream)
//! ```
//!
//! The reverse direction verifies before it returns: a recipe and a carrier
//! blob reproduce the original bytes only when every commitment opens. The
//! content class and privacy layers of the product sit above this facade;
//! the transfer core itself is deterministic and self-verifying.

#![forbid(unsafe_code)]

pub mod carousel;
pub mod codec;
pub mod frame;
pub mod hash;
pub mod matrix;
pub mod payload;
pub mod pipe;
pub mod png;
pub mod qr_encode;
pub mod receive;
pub mod recipe;
pub mod reemit;
pub mod video;

pub use pipe::{
    concat_round_trip, decode_frames, decode_video, encode_content, encode_qr_video, EncodedPipe,
    EncodedQrVideo, PipeError, PIPE_DEFAULT_BLOCK_LEN,
};
pub use recipe::{ThreeRecipe, ThreeRecipePublic, ThreeRecipeSealed};
