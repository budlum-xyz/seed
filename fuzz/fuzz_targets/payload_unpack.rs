#![no_main]
//! Untrusted bytes fed to the A1 container unpacker: it must refuse, never
//! panic, and never return a body whose digest does not open.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = seed_core::payload::unpack_payload(data);
});
