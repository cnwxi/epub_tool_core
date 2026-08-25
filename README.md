# epub_tool_core

`epub_tool_core` is the platform-independent EPUB processing engine shared by
the Epub Tool desktop and Android applications.

The crate exposes typed task contracts and execution APIs. It does not depend
on Tauri, Android APIs, protobuf, UI state, or platform-specific paths.

## API boundary

Applications construct a `TaskSpec`, provide platform-owned paths through
`EngineConfig`, and receive progress through `TaskReporter`:

```rust
let result = epub_tool_core::run(spec, config, reporter)?;
```

The embedding application is responsible for locating and passing OpenCC
resources, OCR model directories, log paths, and all platform file
permissions.

## Features

The default `font` feature provides desktop font encryption/decryption and OCR
support. Android builds use `default-features = false`, which omits the font
and ONNX Runtime dependency tree while retaining EPUB, image, and text tasks.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --no-default-features
cargo test --locked --all-features
```

On macOS, all-feature commands need a compatible ONNX Runtime supplied through
`ORT_LIB_PATH`. The desktop repository's `xtask verify-ocr-model` command
validates its packaged model against this crate's public OCR verification API.
