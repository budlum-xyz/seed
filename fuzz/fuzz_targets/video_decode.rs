#![no_main]
//! Untrusted bytes fed to the video container decoder: the whole reverse
//! pipe (demux, receive, unpack) must refuse cleanly on arbitrary input.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = seed_core::decode_video(data);
});
