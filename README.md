# KOC OCR CLI

KOC OCR CLI 是一个面向抖音 KOC 数据登记场景的命令行 OCR 工具。它读取指定文件夹内的数字命名图片，识别直播截图或视频截图中的关键字段，并导出 CSV 表格。

作者：Lilith@HUAHaohui

## 适用场景

这个分支专注 CLI，不包含桌面应用。它适合被人工直接运行，也适合后续用 Node.js 包一层，做成 Codex Skill 或其他自动化工具。

典型输入目录：

```text
images/
  191.jpg
  192.png
  193.png
```

文件名建议使用纯数字。工具会按数字顺序处理图片，例如 `2.png` 会排在 `10.png` 前面。

## 支持的截图类型

`live`：直播数据截图，导出 `直播.csv`。

字段：

```text
文件名,开播时间,直播时长,PV,累计观看人数,ACU
```

说明：

- 文件名不带后缀。
- 开播时间会整理成 `YYYY-MM-DD HH:mm:ss`。
- 直播时长会转换成纯秒。
- PV 对应截图中的曝光人数，支持把 `2.4万` 转成 `24000`。
- 累计观看人数对应进房人数。
- ACU 对应平均在线人数。
- 没识别到的值输出 `<NULL>`。

`video`：作品数据截图，导出 `视频.csv`。

字段：

```text
文件名,播放量,点赞量,评论量
```

`auto`：自动判断每张图是直播还是视频。适合混合目录，会同时导出 `直播.csv` 和 `视频.csv`。

## 安装模型

首次使用前推荐先执行 `init` 下载模型：

```bash
koc-ocr-cli init --model medium
```

模型档位：

- `tiny`：体积小，速度快，适合简单截图。
- `small`：速度和效果折中。
- `medium`：默认档位，识别效果优先。

默认模型 CDN：

```text
https://assets.checkpoint321.com/wjm/models/
```

指定模型目录：

```bash
koc-ocr-cli init --model medium --model-dir ./models
```

指定 CDN：

```bash
koc-ocr-cli init --model medium --cdn https://assets.checkpoint321.com/wjm/models/
```

默认模型保存位置：

- macOS：`~/Library/Application Support/koc-ocr-cli/models`
- Windows：`%APPDATA%\koc-ocr-cli\models`
- Linux：`~/.local/share/koc-ocr-cli/models`

## 导出表格

导出直播截图：

```bash
koc-ocr-cli export ./images --type live
```

导出视频截图：

```bash
koc-ocr-cli export ./images --type video
```

自动识别截图类型：

```bash
koc-ocr-cli export ./images --type auto
```

指定输出目录：

```bash
koc-ocr-cli export ./images --type live --output ./out
```

指定模型档位：

```bash
koc-ocr-cli export ./images --type live --model small
```

离线运行，不自动下载模型：

```bash
koc-ocr-cli export ./images --type live --model-dir ./models --no-download
```

只输出 CSV 文件路径，不在终端打印 CSV 内容：

```bash
koc-ocr-cli export ./images --type video --quiet
```

## 命令参考

查看帮助：

```bash
koc-ocr-cli --help
koc-ocr-cli init --help
koc-ocr-cli export --help
```

`init` 参数：

```text
--model <tiny|small|medium>     模型档位，默认 medium
--model-dir <path>              模型保存目录
--cdn <url>                     模型 CDN 地址
```

`export` 参数：

```text
<image_dir>                     图片目录
--type <auto|live|video>        截图类型，默认 auto
--model <tiny|small|medium>     模型档位，默认 medium
--model-dir <path>              模型目录
-o, --output <path>             CSV 输出目录
--cdn <url>                     模型 CDN 地址
--no-download                   缺少模型时不自动下载
--quiet                         不打印 CSV 内容
```

## 输出文件

默认输出到图片目录：

```text
images/
  直播.csv
  视频.csv
  .ocr-cache/
```

`.ocr-cache` 是 OCR 结果缓存。重复处理同一批图片时会直接读取缓存，速度会明显快很多。缓存会根据图片大小、修改时间和模型档位自动失效。

## 性能策略

CLI 复用项目内的 OCR 核心逻辑，并使用适合桌面机器的自适应配置：

- 低核心数机器减少推理线程，避免把系统拖卡。
- 6 核及以上机器启用 OCR 并行识别，提高吞吐。
- 重复图片读取 `.ocr-cache`，避免重复 OCR。

## 构建

本地构建当前平台二进制：

```bash
cargo build --release --bin koc-ocr-cli
```

生成的文件：

```text
target/release/koc-ocr-cli
```

Windows 上生成：

```text
target/release/koc-ocr-cli.exe
```

## GitHub Actions 发布

推送形如 `v2.1.0` 的标签会触发 `.github/workflows/release.yml`。

发布产物：

```text
koc-ocr-cli-macos-arm64
koc-ocr-cli-windows-x64.exe
```

这两个产物都是单文件二进制。使用时下载对应平台文件，放到 PATH 中或直接用绝对路径调用。

## 后续 Node 封装建议

Node 层不需要理解 OCR 细节，只需要调用二进制：

```bash
koc-ocr-cli init --model medium
koc-ocr-cli export <图片目录> --type live --output <输出目录> --quiet
```

推荐 Node 封装读取生成的 CSV，而不是解析终端日志。这样更稳定，也更适合做 Skill。

## 开源项目

本项目复用以下开源项目：

| 名称 | 协议 | 仓库 |
| --- | --- | --- |
| PaddleOCR | Apache-2.0 | https://github.com/PaddlePaddle/PaddleOCR |
| ocr-rs | Apache-2.0 | https://github.com/zibo-chen/rust-paddle-ocr |
| MNN | Apache-2.0 | https://github.com/alibaba/MNN |
| image | MIT 或 Apache-2.0 | https://github.com/image-rs/image |
| clap | MIT 或 Apache-2.0 | https://github.com/clap-rs/clap |
| anyhow | MIT 或 Apache-2.0 | https://github.com/dtolnay/anyhow |
| ureq | MIT 或 Apache-2.0 | https://github.com/algesten/ureq |
| Rust | MIT 或 Apache-2.0 | https://github.com/rust-lang/rust |

## 维护原则

这个分支保持 CLI 优先：

- 复用已有 OCR 和 CSV 导出核心。
- 不引入复杂服务端或 GUI 链路。
- 参数保持稳定，方便脚本和 Node 调用。
- 未识别字段统一输出 `<NULL>`，减少自动化处理分支。
