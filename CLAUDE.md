# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 仓库定位

`epub_tool_core` 是桌面端（Tauri）与 Android 端共用的、平台无关的 Rust EPUB 处理引擎。它位于父工作区的三个仓库之一：本 crate、`../epub_tool_rust`（Tauri 2 + Vue 应用）以及 `../homebrew-tap`。应用通过 Git tag 固定本 crate（`epub_tool_core = { git = "...", tag = "v26.9.21" }`），因此版本号与 tag 属于发布契约的一部分；本地联调时在应用侧用 Cargo `[patch]` 覆盖。

本 crate 不得依赖 Tauri、Android API、protobuf、UI 状态或平台路径。IPC wire contract、Protobuf 定义、`engine_adapter` 以及所有子进程/sidecar 逻辑都位于 `epub_tool_rust`。

工具链由 `rust-toolchain.toml` 固定（Rust 1.97.1，edition 2021）。使用四空格缩进、`cargo fmt`，Clippy 必须通过 `-D warnings`。

## 常用命令

```bash
# 格式检查
cargo fmt --all -- --check

# 平台无关引擎（不含字体/OCR 依赖树）的 lint 与测试
cargo clippy --locked --all-targets --no-default-features -- -D warnings
cargo test --locked --no-default-features

# 字体与 OCR 实现的 lint 与测试
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features

# 运行单个测试（单元测试与实现放在同一文件内）
cargo test --locked --all-features <测试名>
cargo test --locked --all-features -- --nocapture
```

CI（`.github/workflows/ci.yml`）在 Ubuntu 24.04 上按上述顺序执行这四条命令。`--no-default-features` 与 `--all-features` 两轮都必须通过：前者即 Android 的编译配置，任何意外泄漏的 `font` feature 符号都会在这一轮失败。

在 macOS 上，`font` feature 需要链接 ONNX Runtime，并通过 `ORT_LIB_PATH` 指向兼容的静态运行时（参见 `../epub_tool_rust/assets/docs/LOCAL_DEVELOPMENT.md`），桌面端的 `xtask` 目标会负责准备。非 macOS 目标则通过 `Cargo.toml` 中按 target 区分的 `ort` 依赖下载 ONNX Runtime 二进制。

## 对外 API

`src/lib.rs` 即是全部对外契约：

- `run(TaskSpec, EngineConfig, impl TaskReporter) -> Result<TaskResult, EngineError>`：以一个批次执行多个文件。
- `supports(&TaskSpec)`：能力与配置探测，不负责校验文件内容。
- `list_font_targets(&Path)` 与 `verify_ocr_model_dir(&Path)`：仅在 `#[cfg(feature = "font")]` 下提供。

`EngineConfig` 只承载平台拥有的内容：日志路径、OpenCC 词典目录、OCR 模型目录。任务需要的其他平台信息应新增到该结构体，而不是读取环境变量或全局状态。`src/task.rs` 定义类型化的 `TaskType` / `TaskSpec` / `TaskOptions` / `TaskEvent` / `TaskResult`；畸形 options 在构造类型化值时就应被拒绝，不要让 wire/JSON 值流入后端。

## 执行模型

`src/backend/mod.rs` 是批处理驱动，也是阅读代码的起点：

1. `task_for(TaskType)` 选出 `Box<dyn EpubTask>`；在缺少 `font` feature 时字体任务返回 `None`，Android 据此判定不支持这些任务。
2. 创建输出目录、**清空**本次日志，并发出 `task.started`。
3. 对每个输入文件计算 `{stem}{suffix}`，发出 `task.file.started`，执行任务后以 `success`/`skip`/`error` 发出 `task.file.finished`，最后由 `task.finished` 携带完整 `TaskResult`。
4. 失败时只在输出文件于本次运行前不存在的情况下删除它。
5. `status` 取值为 `success`、`partial`（存在错误或跳过）或 `error`。

同文件中的 `EpubTask` trait 定义 `supports_options`、`supports_input`、`output_suffix` 与 `process`。任务只修改内存中的 `EpubWorkspace`；仅当任务返回 `TaskOutcome::Success` 时，驱动才调用 `workspace.mark_generated_by_tool()` 与 `workspace.write()`。返回 `Skip` 表示完全不写出文件。

事件与进度约束直接决定 UI 表现：单个文件的进度必须是有限值且单调不减（`TaskUpdate::progress`），批量进度由文件进度换算而来，因此任务上报回退的百分比属于错误，而非展示问题。

默认输出名为 `{stem}_{task_type}.epub`。`chinese_convert` 覆盖为 `_chinese_convert_tc.epub` / `_chinese_convert_sc.epub`。输入文件名已带当前任务后缀时跳过，不重复处理。

## EPUB workspace 与重写层

`src/backend/epub/workspace.rs`：`EpubWorkspace` 将整本书读入 `BTreeMap<String, Vec<u8>>`，是所有 ZIP 字节操作的唯一位置：

- 校验 `mimetype`（必须存在、未压缩且为首个成员），拒绝重复成员，限制单成员大小、总大小与压缩比，并规范化成员路径（拒绝绝对路径与 `..` 穿越）。
- 成员名优先按 UTF-8 解码，失败后才回退到 `zip` crate 的 CP437 解码。大量 EPUB 生成器没有设置 ZIP language encoding flag，只有这样才能让 OPF 中百分号编码的中文 href 与实际成员名匹配；不要"简化"为 `entry.name()`。
- 通过临时文件 + rename 原子写出，强制 `mimetype` 位于首位且不压缩。
- 提供 `resolve_reference` / `rewrite_reference`，负责百分号编解码并保留 `?query` 与 `#fragment` 后缀。

`src/backend/epub/task_base.rs`：`ParsedBook` 以保守方式解析 OPF——正则提取标签加手写属性解析，不使用 XML DOM。不支持的结构以 `当前 Rust 实现暂不支持` 结尾的消息显式失败，不做猜测。`ensure_all_resources_are_manifested` 拒绝存在未登记资源的书。本地 `md5` 实现是刻意保留的，因为文件名混淆算法必须与原始工具逐字节兼容。

`src/backend/epub/rewrite_engine.rs`：共享的 `Reformat` / `Encrypt` / `Decrypt` 重写逻辑。统一整理为 `OEBPS/{Text,Styles,Images,Fonts,Audio,Video,Misc}` 布局，将 NCX 迁移到 `OEBPS/toc.ncx`，并通过 `META-INF/encryption.xml` 或不安全的 manifest basename 判断加密布局。`reformat_epub` / `encrypt_epub` / `decrypt_epub` 只是薄封装，额外处理"已处于目标状态则跳过"。

`src/encoding.rs`：所有文本成员都经 `decode_epub_text` 读取、`encode_epub_text` 写出。合法 UTF-8 始终优先，即使声明了其他编码；否则依次尝试 BOM、字节嗅探、声明编码。输出时声明统一改写为 UTF-8。

## 字体流水线

仅在默认 `font` feature 下编译。流水线为：EPUB → XHTML/CSS → Stylo computed style → `FontRequest` → `FontFaceResolver` → 字符级字体分配。

- `font_style.rs` 封装 Blitz/Stylo/DOM。**Stylo 是生产环境唯一的 CSS 选择器、级联与计算样式引擎。** 不得新增第二套 cascade、旧规则 fallback，或按节点静默降级。必须保留 family stack、weight、style、stretch、`unicode-range`、多 `src`、来源顺序、继承、变量、`!important` 与复杂选择器语义。
- `encrypt_font.rs` 持有 `FontEncryptionPlan`，由加密、解密、目标扫描（`font_targets.rs`）与 `list_font_targets` 共用。字体扫描、加密与解密必须复用该 plan 及其字符级分配结果，不得各自决定元素或字符使用哪个字体。
- `font_obfuscation.rs` 决定哪些字符参与混淆（标点、符号、空白与控制字符原样保留），并构建随机的一对一码点映射。
- `font_cmap.rs` 只重写 Unicode `cmap` 表，其余 OpenType 表逐字节复制。TTF、OTF、WOFF、WOFF2 必须按原容器格式读取、改写并回写。
- `decrypt_font.rs` 渲染混淆字形并用内置 ONNX OCR 识别，要求 `EngineConfig::ocr_model_dir()` 中同时存在 `inference.onnx` 与 `inference.yml`。低置信度或非单字结果绝不允许猜测替换：必须保留状态码、置信度、Top-K 候选与字形图片，并在正文中输出可见占位符。`inference.yml` 的读取是面向已提交 PaddleOCR 格式的窄解析器，而非通用 YAML 解析器。

应用侧的 `xtask verify-ocr-model` 通过 `verify_ocr_model_dir` 校验打包模型。

## 其他任务族

- 图片：`image_processing.rs` 中以 `ImageMode`（`Compress`、`ToWebp`、`WebpToImage`）参数化的单个 `ImageTask`；同目录下三个小模块只是构造器。转换时必须同时更新 manifest 条目与正文引用；压缩在结果不更小时保留原图。像素数上限为 4000 万。
- 封面：`replace_cover.rs` 更新 manifest 中的封面条目与内容页引用；队列中未选择封面的项跳过。
- 文本：`chinese_convert.rs` 从 `EngineConfig::opencc_dir()` 读取 OpenCC 的 `STPhrases/STCharacters` 或 `TSPhrases/TSCharacters`，按标点与空白切分，并保留资源路径、ID、CSS 与脚本内容。

## 测试

- `tests/core_regression.rs` 在运行时用 `zip`/`image` 生成 EPUB 样本，经公共 API 运行真实任务，并断言事件契约（首事件为 `task.started`，末事件为 `task.finished` 且 `progress == 100.0` 并携带结果）、输出后缀、ZIP 成员与改写后的引用。端到端行为的用例应扩展在此文件。
- 单元测试与实现放在同一文件；字体相关测试覆盖 cmap 重写、混淆规划、Stylo 字体解析与 OCR 后处理，无需加载真实模型。
- `tests/fixtures/opencc/` 提供简繁转换测试所需的词典；`tests/data/ttf-parser-demo.ttf.hex` 与 `tests/fixtures/ocr/inference.yml` 是精简样本。
- 若未把 `opencc_dir` 指向 `tests/fixtures/opencc`，`chinese_convert` 测试会失败。

## 新增任务时需要改动的位置

至少涉及：`src/task.rs` 中的 `TaskType` 与 `TaskOptions`；`src/backend/mod.rs` 中的 `task_for` 分发与能力判定；`src/backend/` 下的新模块；一个 `EpubTask` 实现（含后缀与 options 校验）；以及 `tests/core_regression.rs` 中的回归用例。若任务列表对 wire 可见，还需同步修改应用仓库（Protobuf 枚举/options、`engine_adapter`、前端导航与文档），这些都不在本仓库。

## 约定

- 面向用户的提示、日志与错误信息使用中文，新文案保持一致。
- 本 crate 不得引入解释器后端、生成代码或脚本依赖。
- 保持保守失败：遇到不支持的 EPUB 结构时返回明确错误，而不是尽力改写。
- 提交信息使用 `feat(scope): ...`、`fix(scope): ...`、`chore: ...` 形式。
