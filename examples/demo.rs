//! End-to-end demonstration: content in, recipe plus carrier out, content back.
//!
//! Run with: cargo run --example demo

use seed_core::hash::calculate_hash_bytes;
use seed_core::recipe::three_recipe_digest;
use seed_core::{decode_video, encode_qr_video, PIPE_DEFAULT_BLOCK_LEN};

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn main() {
    // The content to transfer: any bytes will do.
    let content: Vec<u8> = b"budlum seed system: content to recipe, recipe to content"
        .iter()
        .cycle()
        .take(9000)
        .copied()
        .collect();

    // Producer side: pack the content into the optical carrier and take the
    // recipe that describes the stream.
    let enc = encode_qr_video(&content, PIPE_DEFAULT_BLOCK_LEN).expect("encode");
    println!("content size  : {} bytes", content.len());
    println!("frame count   : {}", enc.pipe.frames.len());
    println!("video size    : {} bytes", enc.video_blob.len());
    println!(
        "recipe digest : {}",
        hex(&three_recipe_digest(&enc.pipe.recipe))
    );
    println!("stream commit : {}", hex(&enc.pipe.stream_commitment));

    let recipe = &enc.pipe.recipe;
    println!(
        "recipe        : payload={} k={} block={} total={}",
        hex(&recipe.payload_commitment),
        recipe.carousel.k,
        recipe.carousel.block_len,
        recipe.carousel.total_len
    );

    // Consumer side: take the carrier blob, rebuild the content, verify.
    let (kind, back, video) = decode_video(&enc.video_blob).expect("decode");
    assert_eq!(back, content, "rebuilt content must be byte-exact");
    assert_eq!(video.stream_commitment, enc.pipe.stream_commitment);
    assert_eq!(calculate_hash_bytes(&back), calculate_hash_bytes(&content));
    println!("decoded kind   : {kind:?}");
    println!(
        "result        : {} bytes rebuilt, every commitment opened",
        back.len()
    );
}
