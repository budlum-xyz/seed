//! Generate realistic starting seeds for the fuzz targets.
//!
//! Run with: cargo run --example gen_corpus
//! The seeds are the genuine wire bytes of a small transfer, so the fuzzer
//! starts from valid structures instead of nothing.

use seed_core::{encode_qr_video, PIPE_DEFAULT_BLOCK_LEN};

fn main() {
    let content: Vec<u8> = (0..4096u32).map(|i| (i % 251) as u8).collect();
    let enc = encode_qr_video(&content, PIPE_DEFAULT_BLOCK_LEN).expect("encode");

    std::fs::write("fuzz/corpus/payload_unpack/packed.bin", &enc.pipe.packed).expect("write");
    std::fs::write("fuzz/corpus/video_decode/video.bin", &enc.video_blob).expect("write");

    // First frame, first drop: valid structures for the two finer targets.
    let frame = &enc.pipe.frames[0];
    std::fs::write("fuzz/corpus/frame_unpack/frame.bin", frame).expect("write");
    let drop_start = seed_core::carousel::DROP_HEADER_LEN.min(frame.len());
    std::fs::write("fuzz/corpus/drop_parse/drop.bin", &frame[drop_start..]).expect("write");

    println!(
        "corpus seeds written from a {}-byte transfer",
        content.len()
    );
}
