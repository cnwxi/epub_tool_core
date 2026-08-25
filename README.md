# epub_tool_core

`epub_tool_core` 是 Epub Tool 桌面端与 Android 端共用的、平台无关的 EPUB 处理引擎。

该 crate 提供类型化任务契约和执行 API，不依赖 Tauri、Android API、protobuf、UI 状态或平台特定路径。

## 相关仓库

- [Epub Tool 桌面端](https://github.com/cnwxi/epub_tool_rust)
- [Epub Tool Android 端](https://github.com/cnwxi/epub_tool_android)
- [epub_tool_core 共享 Rust 核心](https://github.com/cnwxi/epub_tool_core)

## API 边界

应用通过 `TaskSpec` 构造任务、通过 `EngineConfig` 提供由平台负责的路径，并通过 `TaskReporter` 接收进度：

```rust
let result = epub_tool_core::run(spec, config, reporter)?;
```

嵌入应用负责定位并传入 OpenCC 资源、OCR 模型目录、日志路径和全部平台文件权限。

## 功能特性

默认 `font` feature 为桌面端提供字体加密、解密和 OCR 支持。Android 构建使用
`default-features = false`，省略字体和 ONNX Runtime 依赖树，同时保留 EPUB、图片和文本任务。

## 开发

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --no-default-features
cargo test --locked --all-features
```

在 macOS 上，启用全部 feature 的命令需要通过 `ORT_LIB_PATH` 提供兼容的 ONNX Runtime。桌面端仓库的
`xtask verify-ocr-model` 命令会使用本 crate 的公开 OCR 校验 API 验证其打包模型。
