# Building Digger

## Requirements

- Rust 1.75+ (edition 2021)
- System libraries for your platform

### Debian / Ubuntu

```bash
sudo apt install pkg-config libfontconfig1-dev
```

## Build

```bash
git clone https://github.com/MotherSphere/D1Gg2r-Private.git
cd D1Gg2r-Private
cargo build --release
```

## Run

```bash
cargo run --release
```

## Tests

```bash
cargo test
```

## Optional: GPU support

```bash
cargo build --release --features gpu
```

This enables NVIDIA GPU monitoring via NVML. AMD and Intel GPUs are detected automatically on Linux without this feature flag.
