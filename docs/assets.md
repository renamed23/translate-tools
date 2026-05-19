# crates/text-hook/assets

下面介绍assets每个文件的作用和用法（具体可以参考一下`xtask/test_assets`）

## config.json

```json
{
  "FONT_FACE": "SimSun",
  "CHAR_SET": 134,
  "FONT_FILTER": [
    "ＭＳ ゴシック",
    "俵俽 僑僔僢僋",
    "MS Gothic"
  ],
  "CHAR_FILTER": [
    64
  ],
  "ENUM_FONT_PROC_CHAR_SET": 128,
  "ENUM_FONT_PROC_PITCH": 1,
  "ENUM_FONT_PROC_OUT_PRECISION": 3,
  "CREATE_FONT_C_HEIGHT": -12,
  "CREATE_FONT_C_WIDTH": 0,
  "CREATE_FONT_C_ESCAPEMENT": 0,
  "CREATE_FONT_C_ORIENTATION": 0,
  "CREATE_FONT_C_WEIGHT": 400,
  "CREATE_FONT_B_ITALIC": 0,
  "CREATE_FONT_B_UNDERLINE": 0,
  "CREATE_FONT_B_STRIKE_OUT": 0,
  "CREATE_FONT_I_OUT_PRECISION": 3,
  "CREATE_FONT_I_CLIP_PRECISION": 2,
  "CREATE_FONT_I_QUALITY": 1,
  "CREATE_FONT_I_PITCH_AND_FAMILY": 49,
  "WINDOW_TITLE": "游戏窗口",
  "HIJACKED_DLL_PATH": "some_path/your_dll.dll",
  "RESOURCE_PACK_NAME": "MOZU_chs",
  "HWBP_REG": "crate::utils::hwbp::HwReg::Dr2",
  "HWBP_TYPE": "crate::utils::hwbp::HwBreakpointType::Execute",
  "HWBP_LEN": "crate::utils::hwbp::HwBreakpointLen::Byte1",
  "HWBP_MODULE": "::core::ptr::null()",
  "HWBP_RVA": 4000000,
  "EMULATE_LOCALE_CODEPAGE": 932,
  "EMULATE_LOCALE_LOCALE": 1041,
  "EMULATE_LOCALE_CHARSET": 128,
  "EMULATE_LOCALE_TIMEZONE": "Tokyo Standard Time",
  "EMULATE_LOCALE_WAIT_FOR_EXIT": false,
  "OVERLAY_TARGET_WINDOW_TEXT": "some_window_text",
  "OVERLAY_TARGET_WINDOW_CLASS_NAME": "some_window_class_name",
  "ENTRY_POINT_RVA": 4000000
}
```

若未开启`disable_forced_font`特性，如果传入字体非`FONT_FILTER`则使用`FONT_FACE`固定字体；若开启了`disable_forced_font`，那么传入字体命中`FONT_FILTER`时才使用`FONT_FACE`，否则使用传入的字体。

> 当未开启`disable_forced_font`特性时，`FONT_FILTER`是白名单；开启后则变成黑名单。

`CHAR_SET`对应于GDI函数的`CharSet`

`ENUM_FONT_PROC_CHAR_SET`，`ENUM_FONT_PROC_PITCH`，`ENUM_FONT_PROC_OUT_PRECISION`用于`EnumFonts`系列函数的回调函数，若未指定则不修改。

`CREATE_FONT_C_HEIGHT`，`CREATE_FONT_C_WIDTH`，`CREATE_FONT_C_ESCAPEMENT`，`CREATE_FONT_C_ORIENTATION`，`CREATE_FONT_C_WEIGHT`，`CREATE_FONT_B_ITALIC`，`CREATE_FONT_B_UNDERLINE`，`CREATE_FONT_B_STRIKE_OUT`，`CREATE_FONT_I_OUT_PRECISION`，`CREATE_FONT_I_CLIP_PRECISION`，`CREATE_FONT_I_QUALITY`，`CREATE_FONT_I_PITCH_AND_FAMILY`用于`CreateFont`和`CreateFontIndirect`系列函数，若未指定则使用传入的原始值。

`CHAR_FILTER`用于过滤一些字符(比如需要定长时的填充字符，注意输入的应该是字符的u16值(只支持BMP))，示例中`@`会被过滤，不会被显示出来

`WINDOW_TITLE`在开启`enable_window_title_override`特性后会被用于覆写游戏标题

`HIJACKED_DLL_PATH`用于指定被劫持的DLL的路径，若未指定，那么默认会在系统目录中寻找。需要开启`enable_dll_hijacking`特性，并将需要劫持的DLL放在`assets/hijacked`目录里(仅限一个)，最终编译的DLL需要手动改名，然后放在游戏EXE所在目录即可完成劫持，此时就不再需要改游戏的导入表了。

`RESOURCE_PACK_NAME`在开启`enable_resource_pack`特性后有效，它代表解压到资源包文件的名字。

开启`auto_apply_1337_patch_on_hwbp_hit`或者`enable_hwbp_from_constants`特性时候，使用如下值
- `HWBP_REG`: 硬件断点的寄存器
- `HWBP_TYPE`: 硬件断点的类型（写/访问/执行）
- `HWBP_LEN`: 硬件断点的长度(1，2，4，8)
- `HWBP_MODULE`：硬件断点的模块，也可以是`::windows_sys::w!("sc.dll")`
- `HWBP_RVA`: 硬件断点的相对虚拟地址（相对于模块）

开启`enable_locale_emulator`时，使用如下值
- `EMULATE_LOCALE_CODEPAGE`: 转区的目标代码页
- `EMULATE_LOCALE_LOCALE`: 转区的目标区域码
- `EMULATE_LOCALE_CHARSET`: 转区的目标CharSet
- `EMULATE_LOCALE_TIMEZONE`: 转区的目标时间区域
- `EMULATE_LOCALE_WAIT_FOR_EXIT`: 等待转区后的进程结束再退出

开启`enable_overlay`时，使用如下值
- `OVERLAY_TARGET_WINDOW_TEXT`：目标窗口（需要overlay的窗口）标题
- `OVERLAY_TARGET_WINDOW_CLASS_NAME`：目标窗口（需要overlay的窗口）窗口类名

开启`enable_delayed_attach`时，使用如下值
- `ENTRY_POINT_RVA`：入口点的相对虚拟地址（相对于模块），如果不指定，则使用PE的入口点RVA


## hook_lists.json

```json
{
  "enable": ["TextOutA"],
  "disable": [
    "ExtTextOutA",
    "ExtTextOutW"
  ]
}
```

哪些钩子会被启用取决于`hook_lists.json`以及开启了哪些feature，可以查看 [featured_hook_lists.json](crates/text-hook/constant_assets/featured_hook_lists.json) 了解。

通过`hook_lists.json`来显式指定哪些钩子会被禁止，以及哪些钩子会被开启。

1. `disable` 列表中的钩子会从任何条件中移除
2. `enable` 列表中的钩子会无条件启用
3. `hook_lists.json`中同一个钩子不能同时出现在 enable 和 disable 中

> 例如，如果开启了`bind_font_manager`特性，那么`CreateFontA`钩子会自动启用，可以通过在`disable`指定`CreateFontA`来移除这个钩子。


## font

`font`目录应该只存放一个字体文件，该字体文件会被内嵌到DLL，需要开启`enable_embedded_font`特性

## mapping.json

```json
{
  "code_page": 932,
  "mapping": {
    "鍄": "丽",
    "饋": "讶",
    "輸": "铛",
    "骼": "吵",
    "鎤": "秽",
    "鵡": "块",
  }
}
```

`code_page`是可选的，将用于函数解码文本，如果未指定，那么会使用`src_encoding`，如果也没有`src_encoding`，那么会使用默认值`0`

`mapping`，字符映射规则，左边是替身字符，右边则是会被映射的字符

## raw_patch & translated_patch

raw_patch文件夹包含需要被替换的文件，translated_patch文件夹包含对应的替换文件

若需使用需要开启`enable_patch`特性

## raw_text & translated_text


```json
[
  {
    "name": "右京",
    "message": "急に衝撃があったと思ったらいきなり机が話しかけてきたんでな。俺も少々驚いたよ。",
  },
  {
    "message": "見る",
  },
]
```

raw文件夹包含如上结构的json文件，translated文件夹包含对应的翻译后的json文件，会将文本嵌入到DLL中，使用原文条目调用`lookup`可以获得相对应的译文条目。

需要开启`enable_text_patch`功能，如果需要翻译exe的对话框以及其他exe的文本，则需要开启`bind_user_interface_patcher`功能，可以使用`extract_text` + `bind_lifecycle_guard`功能来从exe中提取出对话框的文本，提取的文本会输出到dll所在目录的`raw.json`中


## hijacked

该目录应该仅有一个文件，并且是你需要劫持的DLL文件，比如`version.dll`，然后过程宏会自动读入该DLL生成对应的导出函数的代码。编译之后，将`text_hook.dll`改名为被劫持的DLL文件名即可，在这个例子中，就是`version.dll`

DLL会`inline hook`入口点，然后加载被劫持的DLL，并获取导出函数的地址，它通过内联汇编`jmp`指令直接跳转到被劫持的DLL对应的导出函数地址，实现转发功能。

> 不只是系统DLL，实际上只要是无命名修饰的符号（比如C++命名修饰的导出符号并不支持）的DLL都可以劫持，也就是说游戏DLL一般也是可以的，不过需要将原始游戏DLL重命名，然后通过`HIJACKED_DLL_PATH`指定位置即可。比如说，游戏导入表有一个`tools.dll`，我们将`tools.dll`拖到`assets/hijacked`，将`HIJACKED_DLL_PATH`的值改为`./tools2.dll`，编译生成，然后将`text_hook.dll`改名为`tools.dll`并复制到游戏目录，将游戏目录原始的`tools.dll`改名为`tools2.dll`，然后就完成劫持游戏DLL了。

> 补充，也不支持有无名导出符号的DLL（即纯序号导出）

> 推荐使用修改导入表的方式注入DLL（比如使用`CFF Explorer`），因为可以精准影响到你想要影响的EXE，比如`chs`版本

## exe

该目录应该仅有一个文件，并且是游戏的exe，目前主要用于
- 开启 `enable_delayed_attach_static` 的过程宏 `generate_entry_point_hook`，该过程宏会分析PE结构，并确保入口点指令不需要重排，然后在编译期生成占位 trampoline，因此不再需要在运行时解析重排指令。一般来说配合IAT HOOK使用，这样可以完全剔除inline hook的依赖。
- 开启 `enable_iat_hook_with_strip` 的过程宏 `generate_hook_lists`，该过程宏会分析PE结构导入表，剔除未使用的 Featured API HOOK（但是如果 `hook_lists.json` 指定了依然会开启），注意，无法处理序号导出（一般来说很少见）


## x64dbg_1337_patch

该目录应该包含由x64dbg生成的补丁文件，在开启`auto_apply_1337_patch_on_attach`特性后，会在DLL attach的时候进行修补，或者可以只开启`enable_x64dbg_1337_patch`并由自己选择修补时机。

开启`auto_apply_1337_patch_on_hwbp_hit`特性后，会在硬件断点命中时进行修补。

## bitmap_font.json

```json
{
    "font_path": "assets/font/MSGothic_WenQuanYi.ttf",
    "font_size": 24,
    "padding": 2,
    "texture_max_width": 48,
    "chars": "hello world\n你好世界"
}
```

在开启`enable_gl_painter`的时候，过程宏会根据`bitmap_font.json`生成位图字体，其中：
- `font_path`: 字体路径，用于光栅化（支持TTF，OTF）
- `font_size`: 字体大小
- `padding`: 位图每个字符的padding
- `texture_max_width`：位图纹理的最大宽度
- `chars`：需要添加到位图里的字符，内部会进行去重


## vfs_rules.json

VFS (Virtual File System) 规则配置, 用于将源路径重定向到目标路径。对文件访问进行透明拦截, 无需修改游戏逻辑。

```json
[
  {
    "source": "{cwd}/data/**/*.*",
    "target": "{cwd}/data_chs/**/*.*",
    "mode": "fallback",
    "create_dirs": ["{cwd}/data_chs/sub1", "{cwd}/data_chs/sub2"],
    "cfg": "feature = \"enable_resource_pack\""
  },
  {
    "source": "{exe_dir}/save/*.*",
    "target": "{exe_dir}/save_chs/*.*",
    "mode": "force",
    "create_dirs": ["{exe_dir}/save_chs"]
  }
]
```

### 字段说明

- **`source`** (必填): 源路径模式, 用于匹配被拦截的文件路径。支持变量占位符和 glob 通配符。
- **`target`** (必填): 目标路径模板, 匹配成功后替换的目标路径。捕获的通配符内容会填充到模板对应位置。
- **`mode`** (必填): 映射模式, 取值为 `"fallback"` 或 `"force"`。
  - `"fallback"`: 目标文件不存在时回退到源路径, 不做重定向。
  - `"force"`: 无条件重定向, 不管目标文件是否存在。
- **`cfg`** (可选): 条件编译守卫, 值为完整的 cfg 表达式 (如 `feature = "enable_resource_pack"`)。带此字段的规则仅在对应 feature 启用时生效。
- **`create_dirs`** (可选): 字符串数组。规则生效前预创建的目标目录路径。支持变量占位符 (`{cwd}` 等), 禁止 glob 通配符, 分隔符须用 `/`。受 `cfg` 守卫控制。同一规则内的 `create_dirs` 共享该规则的 `cfg`；若不同 `cfg` 的规则指定了同一目录路径, 则在各自的 `cfg` 条件下分别创建（类似 `any(a, b)` 语义）。

### 变量占位符

路径中可以使用以下变量, 运行时自动替换为实际路径:

| 变量 | 说明 |
|------|------|
| `{cwd}` | 当前工作目录 |
| `{temp_dir}` | 系统临时目录 |
| `{exe_dir}` | 游戏可执行文件所在目录 |
| `{resource_pack_dir}` | 资源包解压目录 (需 `enable_resource_pack` 特性) |

> 变量替换后, 路径中的 `\` 会自动统一为 `/`, 匹配时不区分大小写。
>
> 以上四种为编译期白名单，任何未列出的 `{var}` 会触发编译错误。

### Glob 模式

每个路径段 (以 `/` 分隔) 可以为以下三种形式:

| 形式 | 匹配行为 |
|------|----------|
| 字面量段 (如 `data`, `MPX`) | 精确匹配 (不区分大小写) |
| `*` 通配段 (如 `*.png`, `name.*`) | 匹配该层单个路径段, 记 1 个捕获组 |
| `*.*` | 匹配该层含 `.` 的路径段, 记 2 个捕获组 |
| `**` 递归通配 | 匹配零个或多个路径段 (含 `/`), 记 1 个捕获组 |

> `source` 中 `*` 和 `**` 捕获的内容会按顺序填入 `target` 对应位置的 `*`/`**` 中。例如 `source: "a/*/b"` 匹配 `a/foo/b` → `target: "x/*/y"` → 输出 `x/foo/y`。

### 路径校验规则

过程宏在编译期会校验 `source` 和 `target` 的合法性, 以下写法会被拒绝:

- **分隔符**: 必须使用 `/`, 禁止 `\\` (如 `a\\b\\*.png`)
- **递归通配**: 整个模式最多允许一个 `**` (如 `**/**/**` 非法)
- **通配段 `*` 数量**: 每个非 `**` 段最多一个 `*`, 特殊允许 `*.*` (恰好两个 `*`)。
  - 合法: `*.png`, `file.*`, `*.*`
  - 非法: `*.*.*`, `a*b*c`, `**.png` (`**` 本身就是一段, 后面不能再跟 `.png`)
- **字面量段**: 不含 `*` 的段不得出现 `*` 字符
- **捕获组数量一致**: `source` 和 `target` 的捕获组总数必须相等。`*` 记 1 个, `*.*` 记 2 个, `**` 记 1 个。
  - 合法: `source: "a/*/b"` / `target: "x/*/y"` (各 1 个捕获组)
  - 非法: `source: "a/*.*"` / `target: "x/*"` (`source` 有 2 个, `target` 有 1 个)
- **目录路径**: `create_dirs` 中的路径禁止出现 `*` 和 `\\`, 且不能为空。
  - 合法: `{cwd}/data_chs/sub`, `{temp_dir}/cache`
  - 非法: `{cwd}/data_*`, `C:\\data`
- **变量占位符**: `source`, `target`, `create_dirs` 中的所有 `{var}` 占位符仅限以下四种:
  `{cwd}`, `{temp_dir}`, `{exe_dir}`, `{resource_pack_dir}`。
  拒绝未知变量（如 `{foobar}`）、空变量 `{}`、未闭合花括号 `{abc`、孤立花括号 `}`、嵌套 `{a{b}}`。
