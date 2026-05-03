# 项目结构

## 核心组件
- **text-hook**: Windows DLL (cdylib)，游戏进程注入钩子库
- **translate-macros**: proc-macro crate，编译期代码生成
- **xtask**: 构建工具，提供`cargo build-text-hook`等命令

## 模块结构

### text-hook 模块结构
```
src/
├── hook/              # 钩子系统核心
│   ├── components/    # 可插拔组件
│   │   ├── text_mapping.rs    # 文本映射 (bind_text_mapping)
│   │   ├── font_manager.rs    # 字体管理 (bind_font_manager)
│   │   └── *.rs               # 其他组件
│   ├── impls/         # 游戏预设配置
│   │   ├── *.rs       # 其他游戏配置
│   │   └── default_impl.rs      # 默认配置
│   ├── api_hooks/           # API钩子，比如 TextOut 等等
│   └── internal_hooks/      # 内部钩子
├── utils/             # 工具函数
│   ├── exts/         # 扩展trait（优先使用）
│   │   ├── *.rs
│   │   ├── ptr_ext.rs
│   │   └── slice_ext.rs
│   ├── mem/          # 内存操作
│   └── win32.rs      # Windows API封装
├── overlay/          # 透明覆盖层系统 (enable_overlay)
│   ├── egui/         # egui界面组件 (enable_overlay_egui)
│   │   ├── components/       # UI组件
│   │   │   ├── font_property_editor.rs  # 字体属性编辑器
│   │   │   ├── logger.rs     # 日志组件
│   │   │   └── demo.rs       # demo组件
│   │   └── integration.rs    # egui集成
│   └── window.rs     # 覆盖层窗口
├── gl/               # OpenGL绘制器 (enable_gl_painter)
└── *.rs              # 基础功能的实现
```

text-hook 可以看作是由
- 其中一个 `hook/impl`
- 一个或多个组件
构成

其中组件是实现了一个或多个`api_hooks`/`internal_hooks`的实现。


### translate-macros 模块结构
```
src/
├── impls/            # 过程宏实现
│   ├── *.rs          # 其他过程宏
│   ├── generate_constants_from_json.rs # 通过JSON生成RUST常量
│   └── detour/                         # 钩子函数生成
│       └── mod.rs
└── utils/            # 宏工具函数
```

## 构建流程
1. 配置`assets/`目录（mapping.json, config.json等）
2. 编译：`cargo build-text-hook --features <特性>`

## 设计要点
- 编译期代码生成（translate-macros）
- 模块化特性系统
- 多种钩子方式（IAT/Inline）
- `constants::*`的变量通过`generate_constants_from_json`生成，该过程宏读取json并生成常量。


# AI 行为准则 (Rules for AI)

## Rust 代码规范
- **严禁造轮子**。在实现功能前，优先检查现有依赖或标准库
  - 如果修改`text-hook`这个crate的代码，优先使用`crate::Result`，`crate::bail!`，`crate::anyhow!`，使用`crate::debug!`打印错误(类似于println!)
  - 优先使用crate提供的`utils/exts`里提供的拓展trait，优先使用链式调用，而不是直接调用函数
- **依赖版本管理**：如果需要引入新的 crate 且我没有指定版本，**必须先查阅该 crate 的最新版本英文文档**，默认使用最新版。
- **rust版本**: 默认为最新版
- **文档标准**：
  - pub函数，pub结构体等等一系列pub条目需要注释文档
  - 如果是pub unsafe函数，那么还需要`# Safety`

## 测试与脚本调用
- 如果需要测试代码，请使用`cargo check --features default_impl`，或者其他`impl`