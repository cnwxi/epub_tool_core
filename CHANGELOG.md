# 更新日志

### 26.9.21
修复 ZIP 成员使用 UTF-8 文件名但未设置 language encoding 标志时被按 CP437 解码，导致 OPF 中百分号编码的中文 href 无法匹配实际资源。

### 26.8.25
首次发布平台无关的 EPUB 处理核心，提供类型化任务契约、统一执行入口以及 EPUB、图片、文本和可选字体处理能力。<br>
核心不依赖 Tauri、Android API、protobuf、UI 类型或平台路径；由客户端通过 `EngineConfig` 提供资源、日志和平台文件能力。<br>
