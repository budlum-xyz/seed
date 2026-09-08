//! One-shot pipe facade: content to payload to carousel to optical frames to
//! video container, and the reverse way back.
//!
//! Keeps callers from wiring every stage by hand in the common case while
//! each stage stays independently testable. The content class and privacy
//! layers of the product sit above this facade; the transfer core itself is
//! deterministic and self-verifying.

use crate::carousel::{
    oneshot_drop_count, CarouselEncoder, CarouselError, DEFAULT_BLOCK_LEN,
    ONESHOT_REPAIR_PERMILLAGE,
};
use crate::codec::{CodecError, CodecKind, FrameMux};
use crate::frame::{fold_frame_digests, frame_digest, pack_frame, FrameError};
use crate::payload::{pack_payload, payload_commitment, PayloadError, PayloadKind};
use crate::receive::{ProgressiveReceiver, ReceiveError};
use crate::recipe::{three_recipe_digest, ThreeRecipePublic};
use crate::video::{demux_optical_frames, QrVideo, QrVideoError, DEFAULT_FPS};

/// Default block length for the facade: the lab default of the carousel.
pub const PIPE_DEFAULT_BLOCK_LEN: u16 = DEFAULT_BLOCK_LEN;

/// Errors from the facade.
#[derive(Debug)]
pub enum PipeError {
    /// Payload container failure.
    Payload(PayloadError),
    /// Carousel failure.
    Carousel(CarouselError),
    /// Optical frame failure.
    Frame(FrameError),
    /// Carrier / mux failure.
    Codec(CodecError),
    /// Receiver failure.
    Receive(ReceiveError),
    /// Video container failure.
    Video(QrVideoError),
}

impl std::fmt::Display for PipeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Payload(e) => write!(f, "pipe payload: {e}"),
            Self::Carousel(e) => write!(f, "pipe carousel: {e}"),
            Self::Frame(e) => write!(f, "pipe frame: {e}"),
            Self::Codec(e) => write!(f, "pipe codec: {e}"),
            Self::Receive(e) => write!(f, "pipe receive: {e}"),
            Self::Video(e) => write!(f, "pipe video: {e}"),
        }
    }
}

impl std::error::Error for PipeError {}

impl From<PayloadError> for PipeError {
    fn from(e: PayloadError) -> Self {
        Self::Payload(e)
    }
}
impl From<CarouselError> for PipeError {
    fn from(e: CarouselError) -> Self {
        Self::Carousel(e)
    }
}
impl From<FrameError> for PipeError {
    fn from(e: FrameError) -> Self {
        Self::Frame(e)
    }
}
impl From<CodecError> for PipeError {
    fn from(e: CodecError) -> Self {
        Self::Codec(e)
    }
}
impl From<ReceiveError> for PipeError {
    fn from(e: ReceiveError) -> Self {
        Self::Receive(e)
    }
}
impl From<QrVideoError> for PipeError {
    fn from(e: QrVideoError) -> Self {
        Self::Video(e)
    }
}

/// Result of one encode pass.
#[derive(Debug)]
pub struct EncodedPipe {
    /// Packed payload container.
    pub packed: Vec<u8>,
    /// Public recipe (`stream_id` = frame-fold when frames were emitted).
    pub recipe: ThreeRecipePublic,
    /// Optical frames.
    pub frames: Vec<Vec<u8>>,
    /// Stream commitment used to bind frames.
    pub stream_commitment: [u8; 32],
}

impl EncodedPipe {
    /// Source block count `k` locked by the carousel params.
    #[must_use]
    pub const fn pipe_recipe_k(&self) -> u16 {
        self.recipe.carousel.k
    }
}

/// Encode content through payload, carousel and optical frames.
///
/// # Errors
///
/// Any stage failure.
pub fn encode_content(content: &[u8], block_len: u16) -> Result<EncodedPipe, PipeError> {
    let packed = pack_payload(PayloadKind::ContentBytes, content)?;
    let commit = payload_commitment(&packed);
    let enc = CarouselEncoder::new(&packed, block_len)?;
    let stream_commitment = enc.params().stream_commitment(&commit);
    // One-shot handover, not a carousel broadcast: systematic pass plus a
    // repair margin, never the 2k cycle. See `oneshot_drop_count`.
    let n = oneshot_drop_count(enc.params().k, ONESHOT_REPAIR_PERMILLAGE);
    let mut frames = Vec::with_capacity(n as usize);
    let mut digests = Vec::with_capacity(n as usize);
    for seq in 0..n {
        let drop = enc.drop_at(seq);
        digests.push(frame_digest(&stream_commitment, seq, &drop.to_bytes()));
        frames.push(pack_frame(&stream_commitment, &drop)?);
    }
    let fold = fold_frame_digests(&digests)?;
    let recipe = ThreeRecipePublic::new(commit, enc.params(), fold);
    Ok(EncodedPipe {
        packed,
        recipe,
        frames,
        stream_commitment,
    })
}

/// Result of an encode pass that also muxes frames into a video container.
#[derive(Debug)]
pub struct EncodedQrVideo {
    /// Pipe encoding (packed, recipe, optical frames, stream id).
    pub pipe: EncodedPipe,
    /// Video container.
    pub video: QrVideo,
    /// Serialized carrier bytes (what a recipe re-emits as the video object).
    pub video_blob: Vec<u8>,
}

/// Encode content all the way to a serialized video container.
///
/// # Errors
///
/// Any stage failure.
pub fn encode_qr_video(content: &[u8], block_len: u16) -> Result<EncodedQrVideo, PipeError> {
    let pipe = encode_content(content, block_len)?;
    let video = QrVideo::from_optical_frames(
        &pipe.recipe,
        &pipe.stream_commitment,
        &pipe.frames,
        DEFAULT_FPS,
    )?;
    let video_blob = video.to_bytes();
    Ok(EncodedQrVideo {
        pipe,
        video,
        video_blob,
    })
}

/// Decode optical frames back to payload kind + unpacked content bytes.
///
/// # Errors
///
/// Receive / unpack failures.
pub fn decode_frames(
    stream_commitment: &[u8; 32],
    frames: &[Vec<u8>],
) -> Result<(PayloadKind, Vec<u8>), PipeError> {
    let mut rx = ProgressiveReceiver::new(*stream_commitment);
    for fr in frames {
        rx.push_frame(fr)?;
        if rx.is_complete() {
            break;
        }
    }
    Ok(rx.finish_unpacked()?)
}

/// Decode a serialized video container back to content.
///
/// # Errors
///
/// Container, receive or unpack failures.
pub fn decode_video(video_blob: &[u8]) -> Result<(PayloadKind, Vec<u8>, QrVideo), PipeError> {
    let video = QrVideo::from_bytes(video_blob)?;
    let optical = demux_optical_frames(&video)?;
    let (kind, raw) = decode_frames(&video.stream_commitment, &optical)?;
    Ok((kind, raw, video))
}

/// Mux a frame list through `M` and split the blob back, returning the frames
/// the container hands an ordinary reader.
///
/// A container is usable when its own reader finds what its own writer put
/// in, so the pair is offered together.
///
/// # Errors
///
/// Propagates `PipeError` from either half.
pub fn concat_round_trip<M>(frames: &[Vec<u8>]) -> Result<Vec<Vec<u8>>, PipeError>
where
    M: FrameMux + Default,
{
    let mux = M::default();
    let blob = mux.mux(CodecKind::RawFrames, frames)?;
    Ok(mux.split(CodecKind::RawFrames, &blob)?)
}

/// Digest of the pipe's public recipe (what a sealed recipe commits to).
#[must_use]
pub fn recipe_commitment(recipe: &ThreeRecipePublic) -> [u8; 32] {
    three_recipe_digest(recipe)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payload::unpack_payload;

    /// A full round trip: content in, video blob out, content back, byte equal.
    #[test]
    fn content_survives_the_full_pipe() {
        let content: Vec<u8> = (0..1024u32).map(|i| (i % 251) as u8).collect();
        let enc = encode_qr_video(&content, PIPE_DEFAULT_BLOCK_LEN).unwrap();
        let (kind, back, video) = decode_video(&enc.video_blob).unwrap();
        assert_eq!(kind, PayloadKind::ContentBytes);
        assert_eq!(back, content);
        assert_eq!(video.stream_commitment, enc.pipe.stream_commitment);
    }

    /// The recipe alone pins the regeneration: re-emitting from the recipe and
    /// the stored frames reproduces the same stream id fold.
    #[test]
    fn recipe_pins_the_stream() {
        let content = b"tarif iceri, tarif disari; ters yon birebir".repeat(40);
        let enc = encode_content(&content, PIPE_DEFAULT_BLOCK_LEN).unwrap();
        assert_eq!(
            enc.recipe.payload_commitment,
            payload_commitment(&enc.packed)
        );
        let (kind, back) = decode_frames(&enc.stream_commitment, &enc.frames).unwrap();
        assert_eq!(kind, PayloadKind::ContentBytes);
        assert_eq!(back, content);
    }

    /// Frames may arrive with gaps: the fountain receiver finishes from a
    /// subset as long as enough equations survive.
    #[test]
    fn receiver_finishes_with_frame_loss() {
        let content: Vec<u8> = (0..4096u32).map(|i| ((i * 7) % 256) as u8).collect();
        let enc = encode_content(&content, PIPE_DEFAULT_BLOCK_LEN).unwrap();
        let mut kept: Vec<Vec<u8>> = Vec::new();
        for (i, fr) in enc.frames.iter().enumerate() {
            if i % 7 != 3 {
                kept.push(fr.clone());
            }
        }
        let (kind, back) = decode_frames(&enc.stream_commitment, &kept).unwrap();
        assert_eq!(kind, PayloadKind::ContentBytes);
        assert_eq!(back, content);
    }

    /// Unpack stays honest: a tampered packed payload is refused, never
    /// silently returned.
    #[test]
    fn tampered_payload_is_refused() {
        let content = b"degistirilemez icerik".repeat(30);
        let enc = encode_content(&content, PIPE_DEFAULT_BLOCK_LEN).unwrap();
        let mut packed = enc.packed.clone();
        let last = packed.len() - 1;
        packed[last] ^= 0xFF;
        assert!(unpack_payload(&packed).is_err());
    }
}
