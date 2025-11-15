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
cargo build --release --features default_impl
```

编译的DLL在`target/i686-pc-windows-msvc/release`

更多`features`请看[crates/text-hook/Cargo.toml](crates/text-hook/Cargo.toml)

注意，`text-hook`重度依赖于编译期代码生成和运算，所以不太可能对不同游戏复用DLL二进制。~~`text-hook`并没有伪造成系统DLL进行注入的功能，所以需要使用`CFF Explorer`之类的工具修改游戏EXE或者DLL的导入表，将`text-hook`添加进去。~~

现在`text-hook`已支持伪造DLL的功能，详情请看下面对应的说明。

## crates/text-hook/assets

下面介绍assets每个文件的作用和用法

### config.json

```json
{
  "FONT_FACE": "SimSun",
  "CHAR_SET": 134,
  "ENUM_FONTS_PROC_CHAR_SET": 128,
  "FONT_FILTER": [
    "ＭＳ ゴシック",
    "俵俽 僑僔僢僋",
    "MS Gothic"
  ],
  "CHAR_FILTER": [
    64
  ],
  "WINDOW_TITLE": "游戏窗口",
  "ARG1": "v1",
  "HIJACKED_DLL_PATH": "",
}
```

若未开启`enum_font_families`特性，那么则使用`FONT_FACE`固定字体，若开启了`enum_font_families`，那么传入字体是`FONT_FILTER`，则使用`FONT_FACE`，否则使用传入的字体

`CHAR_SET`对应于GDI函数的`CharSet`，创建字体函数也会用`CHAR_SET`对应的代码页对字体名进行解码

`ENUM_FONT_PROC_CHAR_SET`用于`EnumFonts`系列函数的回调函数，一般默认为128即可。

`CHAR_FILTER`用于过滤一些字符(比如需要定长时的填充字符，注意输入的应该是字符的u16值(只支持BMP))，示例中`@`会被过滤，不会被显示出来

`WINDOW_TITLE`在开启`override_window_title`特性后会被用于覆写游戏标题

`ARG1`用于特定的游戏实现

`HIJACKED_DLL_PATH`用于指定被劫持的DLL的路径，若为`""`，那么默认会在系统目录中寻找。需要开启`dll_hijacking`特性，并将需要劫持的DLL放在`assets/hijacked`目录里(仅限一个)，最终编译的DLL需要手动改名，然后放在游戏EXE所在目录即可完成劫持，此时就不再需要改游戏的导入表了。

> 仍然推荐使用修改导入表的方式注入DLL，因为可以精准影响到你想要影响的EXE，比如`chs`版本


### font

`font`目录应该只存放一个字体文件，该字体文件会被内嵌到DLL，需要开启`custom_font`特性

### mapping.json

```json
{
  "code_page": 932,
  "src_encoding": "CP932",
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

`src_encoding`一般由`replacement_tool.exe`生成，可选

`mapping`，字符映射规则，左边是替身字符，右边则是会被映射的字符

### raw & translated

raw文件夹包含需要被替换的文件，translated文件夹包含对应的替换文件，需要注意被替换文件和替换文件的文件长度要相等

若需使用需要开启`patch`或者`default_patch_impl`特性

### hijacked

该目录应该仅有一个文件，并且是你需要劫持的DLL文件，比如`version.dll`，然后过程宏会自动读入该DLL生成对应的导出函数的代码。编译之后，将`text_hook.dll`改名为被劫持的DLL文件名即可，在这个例子中，就是`version.dll`

DLL会`inline hook`入口点，然后加载被劫持的DLL，并获取导出函数的地址，它通过内联汇编`jmp`指令直接跳转到被劫持的DLL对应的导出函数地址，实现转发功能。

> 不只是系统DLL，实际上只要是无命名修饰的符号（比如C++命名修饰的导出符号并不支持）的DLL都可以劫持，也就是说游戏DLL一般也是可以的，不过需要将原始游戏DLL重命名，然后通过`HIJACKED_DLL_PATH`指定位置即可。比如说，游戏导入表有一个`tools.dll`，我们将`tools.dll`拖到`assets/hijacked`，将`HIJACKED_DLL_PATH`的值改为`./tools2.dll`，编译生成，然后将`text_hook.dll`改名为`tools.dll`并复制到游戏目录，将游戏目录原始的`tools.dll`改名为`tools2.dll`，然后就完成劫持游戏DLL了。

> 补充，也不支持有无名导出符号的DLL（即纯序号导出）

### x64dbg_1337_patch

该目录应该包含由x64dbg生成的补丁文件，在开启`apply_1337_patch_on_attach`特性后，会在DLL attach的时候进行修补，或者可以只开启`x64dbg_1337_patch`并由自己选择修补时机。

### raw.json & translated.json

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

`raw.json`和`translated.json`为相同结构的json文件，在开启`text_patch`功能后，会将文本嵌入到DLL中，使用原文条目调用`lookup_name`和`lookup_message`可以获得相对应的译文条目。