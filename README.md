![seed banner](assets/seed-banner.png)

Seed is the transfer core of Broad Universal Database 3.0, published as a
standalone crate so the invention can be tested on its own. Content goes in,
a compact self-describing form comes out, that form travels as QR video
frames together with a recipe, and the recipe plus the carrier reproduce the
original content byte for byte on the other side. If a commitment does not
open, the rebuild is refused. Nothing is ever silently wrong.

## What it does

The interface has two halves. On the producer side you upload content, it is
packed into the optical carrier, and a recipe is generated. The recipe is a
small public description of the stream and the only object you have to keep.
On the consumer side the recipe and the carrier are taken in, the stream is
re-emitted and the content is rebuilt. Every commitment must open, and a
digest mismatch refuses the rebuild instead of returning wrong bytes.

## The pipeline

```text
content bytes
  -> payload      (zlib-if-shrinks container + commitment)
  -> carousel     (systematic fountain drops + repair)
  -> frame        (self-describing optical frames, digest-bound)
  -> matrix / png (ISO QR symbols, deterministic raster)
  -> video        (raw frame carrier with commitment)
  -> recipe       (public or sealed parameters that re-emit the stream)
```

The reverse direction runs the same modules in reverse order, with a
verification at every seam.

| Module | Stage | Role |
| --- | --- | --- |
| `payload` | A1 | `zlib`-if-it-shrinks container with an `orig_len` and a `sha256` commitment |
| `carousel` | A2 | Systematic fountain: source drops first, then seeded XOR repair drops |
| `frame` | A3 | Self-describing, digest-bound optical frame (`0xBD3A`) |
| `matrix` | §7 | ISO QR module matrix in byte mode, error correction L, mask 0 |
| `qr_encode` | §7 | Own ISO/IEC 18004 encoder, versions 1 to 40, no library drift |
| `png` | §7 | Deterministic PNG raster of a QR matrix |
| `video` | A4 | `BDLV` container holding ordered QR-PNG frames plus stream and recipe commitments |
| `codec` | A4 | Channel gate deciding which carriers may hold QR frames without ruining them |
| `recipe` | A5 | `ThreeRecipe`, public or sealed, with its domain-separated digests |
| `reemit` | A6 | Recipe to bit-equal stream regeneration |
| `receive` | A7 | Progressive receiver with fountain peeling and prefix availability |
| `hash` | — | Domain-separated hashing helpers |
| `pipe` | facade | One-shot encode and decode over the whole pipe |
| `lib` | root | Crate root and public re-exports |

## The round trip

![tohum > block > tomurcuk](assets/tohum%20%3E%20block%20%3E%20tomurcuk.png)

Three conversions cover the whole cycle. The last two reuse the same modules
as the first one, only in reverse.

**Content to recipe and QR video.** The payload module wraps the content
with `zlib` when that shrinks it and never otherwise, adds a `BDL3` header
carrying version, flags, kind and `orig_len`, and stores the `sha256` of the
uncompressed original; `payload_commitment` binds the container. The carousel
then splits the packed payload into 200 byte source blocks and emits drops,
first the systematic pass where drop *i* is block *i*, then XOR repair drops
of degree 4 to 24 seeded by the drop sequence. This is a fountain code, so
any large enough subset of drops rebuilds the payload and lost frames do not
matter. Each drop is wrapped in an optical frame carrying `seq`, a
`stream_id` prefix and a frame digest that binds stream commitment, sequence
and drop bytes, which keeps a foreign stream from splicing drops in. Every
frame becomes one ISO QR symbol rasterised into a deterministic PNG, written
by a first party encoder so the modules are identical on every machine and
every future dependency bump. The video module muxes the PNG frames into the
`BDLV` container with fps, frame count and both commitments. Finally the
recipe records `payload_commitment`, the carousel parameters, the folded
frame digest `stream_id` and the block length.

**Recipe to QR video.** `RecipeEmitter` in `src/reemit.rs` takes the public
recipe and the packed A1 bytes whose commitment the recipe pins, then
regenerates the carousel drops and optical frames bit-equal to the original
encode. Those frames pass through the same matrix, png and video stages
again, so the recipe reproduces the same QR video every time. A sealed
recipe must be opened first: its holder needs the full public recipe and the
body, and `open_with` refuses a candidate that does not open the commitment.

**QR video to content.** The decoder opens the `BDLV` blob and checks its
commitments against the recipe, demuxes the PNG frames, decodes each QR
symbol back into an optical frame and refuses any frame whose digest does
not open. The frames feed the progressive receiver, which peels the carousel
with degree 1 drops first and GF(2) elimination on the residual while
exposing prefix availability. The 200 byte blocks are reassembled into the
packed payload, the stored `sha256` is verified, and the content is
decompressed when the flag says so. A commitment that does not open refuses
the rebuild.

Either way the result is byte for byte the original, or the rebuild is
refused. The banner tells the same story in three words: tohum is the
recipe, block is the 200 byte stem, tomurcuk is the content.

## Key constants

| Constant | Value |
| --- | --- |
| Default block length | 200 bytes |
| Oneshot repair permillage | 150 |
| Maximum systematic blocks `k` | 4096 |
| Drop header length | 24 bytes |
| Payload header | 47 bytes |
| Maximum payload content | 64 MiB |
| Compression | zlib, applied only if it shrinks |
| QR mode | byte mode, EC level L, mask 0 |
| Raster | 4 px per module, quiet zone 4 modules |
| Maximum QR payload | 2953 bytes |

## Repository layout

```text
assets/         the repository banners
src/            the transfer core, Rust, zero unsafe, forbid(unsafe_code)
examples/       demo.rs runs a full round trip and prints commitments,
                gen_corpus.rs produces the fuzz corpus seeds
fuzz/           libFuzzer targets over every parser surface
web/            the React interface, Vite and Web Crypto
supply-chain/   cargo-vet store with config, self-audits and imports lock
.quality/       cargo-deny, grype, osv-scanner and typos policies
ops/            SBOM generation script
.github/        the security workflow suite described below
```

## Quick start

On the Rust side the whole suite is five commands. `cargo test` runs 91
tests including the golden wire vectors. `cargo run --example demo` performs
a full round trip and prints the commitments. `cargo fmt --all --check` and
`cargo clippy --all-targets -- -D warnings` must both come back clean, and
`cargo vet --locked` reports the supply chain audits green. The toolchain is
pinned to 1.97.1 in `rust-toolchain.toml`, and that file is the MSRV gate.

```bash
cargo test
cargo run --example demo
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo vet --locked
```

The interface in `web/` is a React application with two panes. The first
turns content into a recipe: you choose or drop a file, the app splits it
into chunks, computes the SHA-256 digest with Web Crypto and renders the
recipe text for you to copy or download. The second turns a recipe back into
content: you paste a recipe, the app verifies the chunk count and the digest
before it rebuilds and downloads the file, and a tampered or incomplete
recipe is refused with the reason. The recipe format is deliberately plain
text with a `SEED-RECIPE-1` header, `NAME`, `SIZE`, `DIGEST`, `CHUNKS`,
`CHUNK` and `END` lines and base32 chunks, so it can travel through any
channel a text file can.

```bash
cd web
npm ci
npm run dev
npm test
npm run build
```

## Design guarantees

The crate is free of unsafe code. `#![forbid(unsafe_code)]` sits at the crate
root, so the compiler refuses to let unsafe appear anywhere in first party
code.

The pipe is deterministic and self verifying. Golden wire vectors pin the
exact bytes of the payload container, the carousel drop wire, the stream
commitment and the PNG digest, so two builds of the same source on any
platform produce the same frames.

Every parser surface has an explicit ceiling: 64 MiB of content, 4096
systematic blocks, 2953 bytes per QR symbol, 50 000 frames per video, 8192 px
per PNG side. An untrusted input therefore cannot grow memory without bound.

Commitments bind the whole path end to end. The payload carries its own
`sha256`, each frame is bound to the stream commitment, and the recipe pins
both, so the rebuild either opens every commitment or refuses.

Four libFuzzer targets exercise the drop, frame, payload and video parsers.

## Security

The repository carries the security suite of the Budlum core repository,
adapted to this crate. Every gate below blocks the merge unless stated
otherwise.

| Layer | Mechanism | Workflow |
| --- | --- | --- |
| Secret scanning | gitleaks over the full git history, pinned binary v8.30.1 with sha256, a canary proving the alarm fires, SARIF to the Security tab | `ci.yml` |
| Secret scanning | Semgrep with `p/rust`, `p/security-audit` and `p/secrets`, plus a canary tree | `semgrep.yml` |
| Advisories | cargo-deny over advisories, bans, licences and sources, with a licence gate canary | `ci.yml` |
| Advisories | grype filesystem scan, pinned to v0.116.0 | `grype.yml` |
| Advisories | OSV-Scanner over every lockfile, with a guard that new lockfiles cannot escape the scan | `osv-scanner.yml` |
| Supply chain trust | cargo-vet with Mozilla, Google, Bytecode Alliance, ISRG and Embark imports; mandatory green; the exemption baseline is documented in `supply-chain/config.toml` | `cargo-vet.yml` |
| Supply chain hygiene | cargo-machete, cargo-mutants mutation testing with a canary, cargo-supply-chain publisher visibility with the ownerless dependency gate | `security-hardening.yml` |
| Supply chain hygiene | cargo-udeps for unused dependencies and cargo-geiger holding first party unsafe at zero | `supply-chain-extra.yml` |
| Change review | dependency-review on every PR diff | `dependency-review.yml` |
| Undefined behaviour | Miri over the whole library on a date pinned nightly | `miri.yml` |
| Fuzzing | four libFuzzer targets over payload, drop, frame and video, with nightly long runs and a corpus cache | `fuzz-nightly.yml`, build check in `ci.yml` |
| Determinism | double run reproducibility and a Linux, macOS and Windows digest comparison with a byte equality gate | `determinism.yml` |
| Determinism | cross architecture runs on x86-64 and arm64 | `security-audit.yml` |
| Backdoor class | Diverse Double Compiling, pinned compiler against stable, canonical commitment comparison | `diverse-double-compiling.yml` |
| CodeQL | security-extended plus security-and-quality with a version controlled filter policy | `security-audit.yml` |
| Scorecard | OpenSSF Scorecard with SARIF upload | `security-audit.yml` |
| Provenance | CycloneDX SBOM signed with a GitHub attestation through Sigstore | `provenance.yml` |
| Tooling trust | zizmor with a canary over every workflow, actionlint, an MSRV pin cross check and pinned tool binaries throughout | `security-audit.yml`, `ci.yml` |
| Typos | typos with a canary proving the scanner reads the tree | `typos.yml` |
| Dependency updates | Dependabot weekly, grouped minor and patch, 7 day cooldown, wire format majors pinned | `dependabot.yml` |

Every gate that could pass vacuously carries a canary, which is a
deliberately broken input the tool must catch. A green run therefore means
the tool actually ran and actually measured.

## License

PolyForm Shield License 1.0.0, the same licence as the Budlum core
repository. See `LICENSE`.

Seed is part of Budlum. The B.U.D. architecture, the transfer pipeline, the
wire formats and the naming belong to Budlum. The core B.U.D. 1.0, 2.0 and
3.0 implementation lives in
[`budlum/bud`](https://github.com/budlum-xyz/budlum/tree/main/bud), and this
repository carries the 3.0 seed transfer core on its own.
