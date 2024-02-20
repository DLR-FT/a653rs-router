{ fenix }:
with fenix; combine [
  latest.rustc
  latest.cargo
  latest.clippy
  latest.rustfmt
  latest.rust-src
  latest.rust-analyzer
  targets.x86_64-unknown-linux-musl.latest.rust-std
  targets.thumbv7m-none-eabi.latest.rust-std
  targets.armv7a-none-eabi.latest.rust-std
]
