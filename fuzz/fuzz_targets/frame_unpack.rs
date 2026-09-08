#![no_main]
//! Untrusted bytes fed to the optical frame unpacker under a zeroed stream
//! commitment: refuses or returns a drop bound to that commitment.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let stream = [0u8; 32];
    let _ = seed_core::frame::unpack_frame(&stream, data);
});
