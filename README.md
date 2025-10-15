# TRANSLATE-TOOLS

## 编译方式

### 编译cli-tools

```ps
cargo build --release --bin len_tool
cargo build --release --bin replacement_tool
```

编译的EXE在`target/release`

### 编译text-hook

```ps
cd crates/text-hook
cargo build --release --features default_hook_impl,generate_full_mapping_data
```

编译的DLL在`target/i686-pc-windows-msvc/release`

更多`features`请看[crates/text-hook/Cargo.toml](crates/text-hook/Cargo.toml)

注意，`text-hook`重度依赖于编译期代码生成和运算，所以不太可能对不同游戏复用DLL二进制。`text-hook`并没有伪造成系统DLL进行注入的功能，所以需要使用`CFF Explorer`之类的工具修改游戏EXE或者DLL的导入表，将`text-hook`添加进去。

## crates/text-hook/assets

下面介绍assets每个文件的作用和用法

### config.json

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
  ]
}
```

若未开启`enum_font_families`特性，那么则使用`FONT_FACE`固定字体，若开启了`enum_font_families`，那么传入字体是`FONT_FILTER`，则使用`FONT_FACE`，否则使用传入的字体

`CHAR_SET`对应于GDI函数的`CharSet`

`CHAR_FILTER`用于过滤一些字符(比如需要定长时的填充字符，注意输入的应该是字符的u16值(只支持BMP))，示例中`@`会被过滤，不会被显示出来


### custom_font.ttf

内嵌到DLL的字体，需要开启`custom_font`特性

### mapping.json

```json
{
  "乙": "掸",
  "メ": "边",
  "ひ": "请",
  "冖": "琐",
  "圄": "灵",
  "わ": "卖",
  "匈": "扩",
  "堊": "诀",
}
```

字符映射规则，左边是替身字符，右边则是会被映射的字符，需要注意替身字符必须是jis0208兼容字符

### translated.json

```json
[
  {
    "name": "未ぁ",
    "message": "ぃ，辛苦了。い今天回来得好ぅう。"
  },
  {
    "name": "司",
    "message": "ぇ，比え快到了，也差不多お决定正式かが了。"
  },
]
```

当开启`generate_full_mapping_data`时，生成脚本会读取该文件，生成完整的映射数据(即不需要使用`MultiBytesToWideChar`，直接就映射转码了，速度很快)

### raw & translated

raw文件夹包含需要被替换的文件，translated文件夹包含对应的替换文件，需要注意被替换文件和替换文件的文件长度要相等

若需使用需要开启`patch`或者`default_patch_impl`特性