# TRANSLATE-TOOLS

## 编译方式

### 编译text-hook

```ps
cd crates/text-hook
cargo build --release --features default_impl
```

编译的DLL在`target/i686-pc-windows-msvc/release`

更多`features`请看[crates/text-hook/Cargo.toml](crates/text-hook/Cargo.toml)

注意，`text-hook`重度依赖于编译期代码生成和运算，所以不太可能对不同游戏复用DLL二进制。

## crates/text-hook/assets

下面介绍assets每个文件的作用和用法

### config.json

```json
{
  "FONT_FACE": "SimSun",
  "CHAR_SET": 134,
  "ENUM_FONT_PROC_CHAR_SET": 128,
  "ENUM_FONT_PROC_PITCH": 1,
  "ENUM_FONT_PROC_OUT_PRECISION": 3,
  "FONT_FILTER": [
    "ＭＳ ゴシック",
    "俵俽 僑僔僢僋",
    "MS Gothic"
  ],
  "CHAR_FILTER": [
    64
  ],
  "WINDOW_TITLE": "游戏窗口",
  "HIJACKED_DLL_PATH": "some_path/your_dll.dll",
  "REDIRECTION_SRC_PATH": "DATA2.TCD",
  "REDIRECTION_TARGET_PATH": "DATA_chs.TCD",
}
```

若未开启`enum_font_families`特性，如果传入字体非`FONT_FILTER`则使用`FONT_FACE`固定字体，若开启了`enum_font_families`，那么传入字体是`FONT_FILTER`，则使用`FONT_FACE`，否则使用传入的字体

> 当未开启`enum_font_families`特性时，`FONT_FILTER`是白名单，开启时则变成黑名单了

`CHAR_SET`对应于GDI函数的`CharSet`

`ENUM_FONT_PROC_CHAR_SET`，`ENUM_FONT_PROC_PITCH`，`ENUM_FONT_PROC_OUT_PRECISION`用于`EnumFonts`系列函数的回调函数，若未指定则不修改。

`CHAR_FILTER`用于过滤一些字符(比如需要定长时的填充字符，注意输入的应该是字符的u16值(只支持BMP))，示例中`@`会被过滤，不会被显示出来

`WINDOW_TITLE`在开启`override_window_title`特性后会被用于覆写游戏标题

`HIJACKED_DLL_PATH`用于指定被劫持的DLL的路径，若未指定，那么默认会在系统目录中寻找。需要开启`dll_hijacking`特性，并将需要劫持的DLL放在`assets/hijacked`目录里(仅限一个)，最终编译的DLL需要手动改名，然后放在游戏EXE所在目录即可完成劫持，此时就不再需要改游戏的导入表了。

> 推荐使用修改导入表的方式注入DLL（比如使用`CFF Explorer`），因为可以精准影响到你想要影响的EXE，比如`chs`版本


### hook_lists.json

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

> 例如，如果开启了`text_hook`特性，那么`CreateFontA`钩子会自动启用，可以通过在`disable`指定`CreateFontA`来移除这个钩子。


### font

`font`目录应该只存放一个字体文件，该字体文件会被内嵌到DLL，需要开启`custom_font`特性

### mapping.json

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

### raw & translated

raw文件夹包含需要被替换的文件，translated文件夹包含对应的替换文件，需要注意被替换文件和替换文件的文件长度要相等

若需使用需要开启`patch`或者`default_patch_impl`特性

### raw_text & translated_text


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

raw文件夹包含如上结构的json文件，translated文件夹包含对应的翻译后的json文件，会将文本嵌入到DLL中，使用原文条目调用`lookup_name`和`lookup_message`可以获得相对应的译文条目。

需要开启`text_patch`功能，如果需要翻译exe的对话框以及其他exe的文本，则同时需要开启`window_hook`功能，可以使用`text_extracting`功能来从exe中提取出对话框的文本，提取的文本会输出到dll所在目录的`raw.json`中


### hijacked

该目录应该仅有一个文件，并且是你需要劫持的DLL文件，比如`version.dll`，然后过程宏会自动读入该DLL生成对应的导出函数的代码。编译之后，将`text_hook.dll`改名为被劫持的DLL文件名即可，在这个例子中，就是`version.dll`

DLL会`inline hook`入口点，然后加载被劫持的DLL，并获取导出函数的地址，它通过内联汇编`jmp`指令直接跳转到被劫持的DLL对应的导出函数地址，实现转发功能。

> 不只是系统DLL，实际上只要是无命名修饰的符号（比如C++命名修饰的导出符号并不支持）的DLL都可以劫持，也就是说游戏DLL一般也是可以的，不过需要将原始游戏DLL重命名，然后通过`HIJACKED_DLL_PATH`指定位置即可。比如说，游戏导入表有一个`tools.dll`，我们将`tools.dll`拖到`assets/hijacked`，将`HIJACKED_DLL_PATH`的值改为`./tools2.dll`，编译生成，然后将`text_hook.dll`改名为`tools.dll`并复制到游戏目录，将游戏目录原始的`tools.dll`改名为`tools2.dll`，然后就完成劫持游戏DLL了。

> 补充，也不支持有无名导出符号的DLL（即纯序号导出）

### x64dbg_1337_patch

该目录应该包含由x64dbg生成的补丁文件，在开启`apply_1337_patch_on_attach`特性后，会在DLL attach的时候进行修补，或者可以只开启`x64dbg_1337_patch`并由自己选择修补时机。