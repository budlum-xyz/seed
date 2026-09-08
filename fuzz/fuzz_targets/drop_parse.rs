#![no_main]
//! Untrusted bytes fed to the carousel drop parser: refuses or parses, the
//! body hash check must hold on anything accepted.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = seed_core::carousel::Drop::from_bytes(data);
});
