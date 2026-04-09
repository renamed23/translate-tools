# AI 行为准则 (Rules for AI)

## 1. Rust 代码规范
- **严禁造轮子**。在实现功能前，优先检查现有依赖或标准库
  - 如果修改`text-hook`这个crate的代码，优先使用`crate::Result`，`crate::bail!`，`crate::anyhow!`，使用`crate::debug!`打印错误(类似于println!)
  - 优先使用crate提供的`utils/exts`里提供的拓展trait，优先使用链式调用，而不是直接调用函数
- **依赖版本管理**：如果需要引入新的 crate 且我没有指定版本，**必须先查阅该 crate 的最新版本英文文档**，默认使用最新版。
- **rust版本**: 默认为最新版
- **文档标准**：
  - pub函数，pub结构体等等一系列pub条目需要注释文档
  - 如果是pub unsafe函数，那么还需要`# Safety`

## 2. 测试与脚本调用
- 如果需要测试代码，请使用`cargo check --features default_impl`，或者其他`impl`
