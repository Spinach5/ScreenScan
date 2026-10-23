<p align="center">
  <img src="assets/tray-icon.png" width="88" alt="ScreenScan 图标" />
</p>

<h1 align="center">ScreenScan</h1>

<p align="center">
  跨平台截屏 · 标注 · 贴图 · 扫码<br />
  <sub>Windows 10+ / macOS 12.3+ / Linux（X11 与 Wayland）</sub>
</p>

<p align="center">
  <a href="https://github.com/Spinach5/ScreenScan/releases">下载</a> ·
  <a href="#从源码构建">构建</a> ·
  <a href="#使用">使用</a> ·
  <a href="#配置">配置</a> ·
  <a href="#已知限制">已知限制</a> ·
  <a href="LICENSE">MIT License</a>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT" /></a>
  <img src="https://img.shields.io/badge/rust-1.95%2B-orange.svg" alt="Rust 1.95+" />
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg" alt="Platforms" />
</p>

ScreenScan 是一个用 Rust 编写的桌面截图工具，交互对标 Snipaste / Flameshot：从系统托盘、全局快捷键或命令行触发后，屏幕画面立即冻结，拖拽框选区域，在选区上涂画、擦除，然后复制到剪贴板、保存为图片、贴成悬浮窗，或识别选区中的二维码 / 条码。

核心渲染基于 **winit + egui + OpenGL**，不依赖 GTK/Qt；Windows、macOS、Linux（X11 与 Wayland）使用同一套代码。

## 目录

- [特性](#特性)
- [平台支持](#平台支持)
- [安装](#安装)
  - [预编译包](#预编译包)
  - [从源码构建](#从源码构建)
  - [本地打包](#本地打包)
- [使用](#使用)
  - [启动方式](#启动方式)
  - [截图流程](#截图流程)
  - [选区操作](#选区操作)
  - [工具栏](#工具栏)
  - [贴图窗口](#贴图窗口)
  - [条码 / 二维码扫描](#条码--二维码扫描)
  - [托盘菜单](#托盘菜单)
  - [快捷键](#快捷键)
  - [命令行参数](#命令行参数)
  - [首选项](#首选项)
- [配置](#配置)
  - [配置文件位置](#配置文件位置)
  - [配置示例](#配置示例)
  - [配置项说明](#配置项说明)
  - [文件名模板](#文件名模板)
  - [日志](#日志)
- [已知限制](#已知限制)
- [架构](#架构)
  - [项目结构](#项目结构)
  - [设计要点](#设计要点)
  - [技术栈](#技术栈)
- [开发与发布](#开发与发布)
- [故障排查](#故障排查)
- [贡献](#贡献)
- [许可证](#许可证)
- [致谢](#致谢)

## 特性

- **冻结截图式覆盖层**：触发后先抓取全部显示器画面，再在其上绘制暗色蒙层和选区。不依赖透明窗口，规避 macOS 透明窗口变黑、Wayland 无法绝对定位等问题。
- **多显示器与混合 DPI**：每块显示器一个覆盖层窗口；鼠标所在屏幕保持明亮，其余屏幕变暗；选区与导出按物理像素处理，笔画按逻辑点记录，混合缩放比例下不偏移。
- **完整的选区交互**：默认全屏选区，拖拽创建新选区；8 个方向缩放（最小 8 px）、拖拽内部移动、在选区外拖拽重新框选；实时显示选区像素尺寸。
- **标注工具**：画笔、涂鸦、橡皮擦；可调颜色与线宽（1–30 px）；按笔画擦除；完整的撤销 / 重做。
- **四种输出动作**：复制到剪贴板、另存为 PNG / JPEG / WebP、贴图悬浮窗、条码 / 二维码识别。
- **二维码 / 条码扫描**：后台识别，自动尝试原图、反色、2× 放大三轮预处理；支持 QR、Data Matrix、PDF417、Aztec、Code 128、Code 39、EAN-13；识别结果自动复制到剪贴板。
- **常驻托盘与首选项**：托盘菜单、开机启动、界面字体、自定义托盘图标、日志级别、文件名模板、托盘与首选项中英文界面。
- **跨平台发布**：CI 自动构建 Windows（zip / NSIS 安装包）、macOS（.app / dmg / zip）、Linux（deb / rpm / pacman / AppImage / tar.xz），覆盖 x86_64 与 arm64。

## 平台支持

| 能力 | Windows 10/11 | macOS 12.3+ | Linux X11 | Linux Wayland |
| --- | --- | --- | --- | --- |
| 屏幕捕获 | DXGI Desktop Duplication | ScreenCaptureKit | X11 | xdg-desktop-portal + PipeWire |
| 多屏覆盖层 | 支持 | 支持 | 支持 | 支持（逐输出全屏） |
| 全局快捷键 | 支持 | 支持 | 支持 | 不可用（改用桌面快捷键调用 CLI） |
| 贴图始终置顶 | 支持 | 支持 | 支持 | 受限（普通无边框窗口） |
| 系统托盘 | 支持 | 支持 | 支持（StatusNotifierItem） | 支持（依赖桌面环境） |
| 截屏授权 | 无需 | 屏幕录制权限 | 无需 | 门户授权 |

> Linux 下使用纯 Rust 的 `ksni` 实现 StatusNotifierItem，不依赖 GTK；但托盘是否显示取决于桌面环境是否提供 StatusNotifier 宿主。

## 安装

### 预编译包

从 [GitHub Releases](https://github.com/Spinach5/ScreenScan/releases) 下载对应平台的产物：

| 平台 | 文件 | 说明 |
| --- | --- | --- |
| Windows | `ScreenScan-<版本>-windows-<arch>.zip` | 免安装，直接运行 `ScreenScan.exe` |
| Windows | `ScreenScan-<版本>-windows-<arch>-setup.exe` | 当前用户安装，无需管理员权限 |
| macOS | `ScreenScan-<版本>-macos-<arch>.dmg` / `.zip` | arm64 与 x86_64 分别构建 |
| Linux | `screenscan_<版本>_<arch>.deb` | Debian / Ubuntu 等 |
| Linux | `screenscan-<版本>-1.<arch>.rpm` | Fedora / RHEL / openSUSE 等 |
| Linux | `screenscan-<版本>-1-<arch>.pkg.tar.zst` | Arch Linux / Manjaro |
| Linux | `ScreenScan-<版本>-linux-<arch>.AppImage` | 免安装，`chmod +x` 后直接运行 |
| Linux | `ScreenScan-<版本>-linux-<arch>.tar.xz` | 便携包 |

常用安装命令：

```bash
# Debian / Ubuntu
sudo apt install ./screenscan_*_amd64.deb

# Fedora / RHEL
sudo dnf install ./screenscan-*.rpm

# Arch Linux
sudo pacman -U screenscan-*.pkg.tar.zst

# AppImage
chmod +x ScreenScan-*-linux-x86_64.AppImage
./ScreenScan-*-linux-x86_64.AppImage
```

macOS 产物使用 ad-hoc 签名，**未做 Apple 公证**。首次打开若被 Gatekeeper 拦截，可在「系统设置 → 隐私与安全性」中选择「仍要打开」，或执行：

```bash
xattr -dr com.apple.quarantine /Applications/ScreenScan.app
```

Windows 安装包安装在 `%LOCALAPPDATA%\Programs\ScreenScan`，卸载信息写入当前用户注册表，不会请求管理员权限。

### 从源码构建

#### 环境要求

- Rust **1.95 或更新版本**（项目使用 edition 2024，并在 `Cargo.toml` 中声明 `rust-version = "1.95"`）
- Linux 需要额外的系统开发库：

```bash
# Debian / Ubuntu
sudo apt update
sudo apt install -y \
  pkg-config libpipewire-0.3-dev libgbm-dev libdrm-dev libegl1-mesa-dev \
  libclang-dev libwayland-dev libxkbcommon-dev libxcb1-dev \
  libxcb-randr0-dev libxcb-shm0-dev libxcb-shape0-dev \
  libxcb-xfixes0-dev libxcb-sync-dev

# Fedora
sudo dnf install -y pkgconf-pkg-config pipewire-devel mesa-libgbm-devel \
  libdrm-devel mesa-libEGL-devel clang wayland-devel libxkbcommon-devel libxcb-devel

# Arch Linux
sudo pacman -S --needed base-devel pkgconf pipewire mesa libdrm clang \
  wayland libxkbcommon libxcb
```

`libclang` 是构建期依赖（bindgen 需要），运行时不需要。

#### 构建与运行

```bash
git clone https://github.com/Spinach5/ScreenScan.git
cd ScreenScan

cargo build --release --locked
cargo run --release
```

构建产物为 `target/release/ScreenScan`（注意首字母大写；Linux 安装包会将其安装为小写的 `screenscan`）。

```bash
# 常驻托盘
./target/release/ScreenScan

# 截一次图后退出
./target/release/ScreenScan -c

# 直接打开首选项
./target/release/ScreenScan --preferences
```

#### 测试

```bash
cargo test --locked
cargo fmt --all -- --check
cargo clippy --all-targets
```

### 本地打包

Linux 使用 `packaging/linux/package.sh` 生成 deb、rpm、pacman、AppImage 和 tar.xz：

```bash
cargo build --release --locked
bash packaging/linux/package.sh target/release/ScreenScan 0.1.0 x86_64 dist
```

脚本依赖：

- [nfpm](https://github.com/goreleaser/nfpm)（生成 deb / rpm）
- `bsdtar`、`zstd`（生成 pacman 包）
- `rsvg-convert`（把 SVG 图标转成 PNG）
- 脚本会自动下载 `appimagetool`

Windows 安装包使用 NSIS：

```powershell
makensis /DVERSION=0.1.0 /DARCH=x86_64 packaging/windows/installer.nsi
```

> `installer.nsi` 会打包当前目录下的 `ScreenScan.exe`。CI 会先把可执行文件和脚本复制到临时目录再调用 `makensis`，参考 `.github/workflows/release.yml`。

## 使用

### 启动方式

- **无参数启动**：不显示主窗口，直接驻留系统托盘，并尝试注册全局快捷键。
- **托盘菜单**：左键或右键点击托盘图标打开菜单，选择「截屏」。
- **全局快捷键**：非 Wayland 且常驻运行时，`Ctrl+Shift+S` 截屏，`Ctrl+Shift+Q` 退出。
- **命令行**：`screenscan -c` 执行一次截图，适合绑定到桌面环境的自定义快捷键（Wayland 推荐方式）。
- **首选项**：托盘菜单「首选项」，或启动时加 `--preferences`。

### 截图流程

1. 触发截屏后，后台线程抓取所有显示器画面，主线程为每块屏幕创建一个无边框全屏覆盖层。
2. 画面被冻结并压暗，鼠标所在的显示器保持明亮，其余显示器整屏变暗。
3. 默认选区为当前显示器全屏。拖拽可重新框选；选区确认后出现浮动工具栏。
4. 选择画笔 / 涂鸦 / 橡皮擦进行标注，可随时撤销、重做。
5. 点击「复制」「保存」「贴图」或「扫描」执行动作。

复制、保存成功后会结束整个截图会话并关闭所有覆盖层；贴图在悬浮窗创建成功后关闭覆盖层；扫描不会关闭覆盖层，识别结果以顶部提示的形式显示。

### 选区操作

| 操作 | 效果 |
| --- | --- |
| 左键拖拽 | 创建新选区；拖拽距离过小时视为单击 |
| 左键单击（未拖拽） | 确认当前的默认全屏选区 / 已有选区 |
| 拖拽边或角 | 8 方向缩放，最小尺寸 8 物理像素 |
| 拖拽选区内部 | 平移选区，自动约束在屏幕内 |
| 在选区外拖拽 | 重新框选 |
| 右键（非绘制状态） | 恢复为默认全屏选区 |
| 右键（绘制状态） | 撤销上一笔 |
| `Esc` | 绘制状态退回选区调整；其他状态退出截图 |
| 鼠标进入其他屏幕 | 切换活动屏幕（拖拽过程中不会切换） |

### 工具栏

工具栏显示在选区下方居中；下方空间不足时自动翻转到选区上方，并始终限制在当前屏幕内。

| 控件 | 说明 |
| --- | --- |
| 画笔 | 实线绘制，使用当前颜色与线宽 |
| 涂鸦 | 自由手绘，使用当前颜色，固定 2 px 线宽 |
| 橡皮擦 | 擦除与擦除路径相交的**整条笔画**，线宽可调 |
| 颜色 | 取色器选择标注颜色 |
| 线宽 | 1–30 px 滑块 |
| 撤销 / 重做 | 以笔画为单位；一次擦除操作可整体撤销 |
| 复制 | 合成标注后写入图片剪贴板 |
| 保存 | 弹出系统保存对话框，支持 PNG / JPEG / WebP |
| 贴图 | 创建显示合成结果的悬浮窗口 |
| 扫描 | 识别选区中的二维码 / 条码，结果复制到剪贴板 |

再次点击当前激活的工具，会退出绘制模式并回到选区调整（鼠标）模式。绘制操作仅限选区内部。

### 贴图窗口

贴图窗口是无边框、不可缩放的独立窗口，显示带标注的合成图像：

- 在图像任意位置（关闭按钮除外）按住左键可拖动窗口。
- 点击右上角 `X` 关闭。
- Windows / macOS / X11 下保持置顶；Windows 上窗口失去焦点时会重新断言置顶。
- Wayland 会话下无法通用置顶，窗口降级为普通无边框窗口。
- 一次性模式（`-c`）下，最后一个贴图关闭后应用才会退出；常驻模式不受影响。

### 条码 / 二维码扫描

扫描会裁剪选区、合成标注，然后在后台线程中解码。为避免漏检，会依次尝试：

1. 原始图像；
2. 颜色反色；
3. 最近邻 2× 放大。

支持 QR Code、Data Matrix、PDF417、Aztec、Code 128、Code 39、EAN-13，并启用 `TRY_HARDER`。识别成功后将文本写入剪贴板并提示「已复制到剪贴板」；失败提示「未识别到条码」。扫描期间覆盖层保持打开。

### 托盘菜单

| 菜单项 | 行为 |
| --- | --- |
| 截屏 | 抓取全部显示器并进入截图流程 |
| 贴图 | 入口已预留，当前版本无操作；请使用截图工具栏中的「贴图」 |
| 清空截图记录 | 删除 `~/.screenscan/history` 目录（截图历史存储目前为预留接口） |
| 首选项 | 打开设置窗口 |
| 退出 | 退出应用 |

### 快捷键

| 快捷键 | 作用域 | 说明 |
| --- | --- | --- |
| `Ctrl+Shift+S` | 全局（Windows / macOS / X11） | 开始截图 |
| `Ctrl+Shift+Q` | 全局（Windows / macOS / X11） | 退出应用 |
| `Esc` | 截图覆盖层 | 绘制中退回选区调整，否则退出截图 |

Wayland 不允许应用注册全局快捷键。请把 `screenscan -c` 绑定到桌面环境的自定义快捷键，例如：

- **GNOME**：设置 → 键盘 → 自定义快捷键 → 命令填写 `screenscan -c`
- **KDE Plasma**：系统设置 → 快捷键 → 添加命令/URL → 命令填写 `screenscan -c`

如果运行的是源码构建产物，请使用其绝对路径，例如 `/path/to/ScreenScan/target/release/ScreenScan -c`。

### 命令行参数

| 参数 | 说明 |
| --- | --- |
| 无参数 | 驻留托盘，注册全局快捷键（可用时） |
| `-c` / `--capture` / `--once` | 执行一次截图后退出；不创建托盘、不注册全局快捷键 |
| `--preferences` | 启动后直接打开首选项窗口（可与常驻模式配合） |

参数可以重复出现，但一次性模式下 `--preferences` 不会生效。程序没有内建的 `--help` / `--version` 输出，版本号显示在首选项的「关于」页。

### 首选项

所有修改即时生效并自动写回 `config.ini`。

| 标签页 | 内容 |
| --- | --- |
| 常规 | 开机启动；保存设置前备份配置；查看并迁移配置文件位置 |
| 界面 | 选择系统中已安装的界面字体；使用默认或自定义托盘图标（支持 PNG / JPG / WebP / BMP / GIF，过大的图片会缩放） |
| 输出 | 日志级别（无 / 调试 / 普通 / 错误）、打开日志目录、保存文件名模板及预览 |
| 语言 | 简体中文 / English |
| 更新 | 占位功能，后续版本提供检查更新 |
| 关于 | 版本与版权信息 |

「自动备份」备份的是配置文件（`config.ini.bak`），不是截图文件。

开机启动的实现方式：

- **Linux**：写入 `~/.config/autostart/screenscan.desktop`（遵循 `XDG_CONFIG_HOME`）
- **macOS**：写入 `~/Library/LaunchAgents/com.spinach5.screenscan.plist`
- **Windows**：写入 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`

## 配置

### 配置文件位置

| 用途 | 默认路径 |
| --- | --- |
| 配置文件 | `~/.screenscan/config.ini`（Linux/macOS）；`%USERPROFILE%\.screenscan\config.ini`（Windows） |
| 日志 | `~/.screenscan/logs/screenscan-YYYY-MM-DD.log` |
| 截图历史 | `~/.screenscan/history`（存储接口预留，当前仅支持清空） |

如果通过首选项修改了配置文件位置，应用会把配置写入新目录下的 `config.ini`，并在默认路径保留一个引导文件，内容形如：

```ini
[storage]
config_path=/path/to/your/config.ini
```

默认位置的文件只作为「指向自定义配置」的入口；若自定义配置不存在，会回退到默认配置。

### 配置示例

```ini
[general]
autostart=false
auto_backup=true

[interface]
; 可选：自定义界面字体
; font_path=/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc
; font_index=0
; font_family=Noto Sans CJK SC
tray_icon=default

[output]
log_level=info
filename_format=screenscan_$yyyy-MM-dd_HH-mm-ss$.png

[appearance]
language=zh-CN
```

### 配置项说明

| 节 / 键 | 取值 | 默认值 | 说明 |
| --- | --- | --- | --- |
| `general.autostart` | `true` / `false` | `false` | 登录后自动在托盘驻留 |
| `general.auto_backup` | `true` / `false` | `false` | 保存前把旧配置备份为 `config.ini.bak` |
| `interface.font_path` | 字体文件路径 | 空 | 自定义界面字体；支持 `~` 展开 |
| `interface.font_index` | 整数 | `0` | TTC / OTC 字体集合中的索引 |
| `interface.font_family` | 字体族名 | 空 | 仅用于展示与记录 |
| `interface.tray_icon` | `default` / `custom` | `default` | 托盘图标来源 |
| `interface.tray_icon_path` | 图片路径 | 空 | 指定 `custom` 时使用 |
| `output.log_level` | `off` / `debug` / `info` / `error` | `info` | 终端与文件日志级别 |
| `output.filename_format` | 文件名模板 | `screenscan_$yyyy-MM-dd_HH-mm-ss$.png` | 保存对话框的默认文件名 |
| `appearance.language` | `zh-CN` / `en-US` | `zh-CN` | 界面语言 |
| `storage.config_path` | 绝对路径 | 无 | 仅出现在默认路径的引导文件中 |

INI 解析器会忽略以 `;` 或 `#` 开头的注释，节名与键名不区分大小写。

### 文件名模板

时间占位符使用 `$...$` 包裹，只在包裹范围内替换：

| 占位符 | 含义 | 示例 |
| --- | --- | --- |
| `yyyy` | 四位年份 | `2026` |
| `yy` | 两位年份 | `26` |
| `MM` | 月份 | `10` |
| `dd` | 日期 | `24` |
| `HH` | 小时（24 小时制） | `15` |
| `mm` | 分钟 | `08` |
| `ss` | 秒 | `39` |
| `SSS` | 毫秒 | `123` |

示例：

```ini
filename_format=shot_$yyyyMMdd-HHmmss$.png
# → shot_20261024-150839.png
```

模板只决定保存对话框中的默认文件名；最终格式由用户在保存对话框中选择的扩展名决定。在首选项的「输出」页可以实时预览，也可以一键恢复默认模板。

### 日志

- 日志同时输出到终端和文件，按天写入 `screenscan-YYYY-MM-DD.log`。
- 启动时自动清理超过 **14 天** 的日志文件。
- 通过首选项的「日志级别」调整，或在启动时设置 `RUST_LOG` 环境变量覆盖：

```bash
RUST_LOG=debug ./target/release/ScreenScan
```

- 日志中会记录版本、操作系统、会话类型、可执行文件与配置路径、显示器信息、抓屏与动作结果，便于排查问题。
- 在首选项的「输出」页可直接打开日志目录。

## 已知限制

### Wayland

- **全局快捷键不可用**：Wayland 不提供通用全局快捷键接口。使用 `screenscan -c` 配合桌面环境自定义快捷键。
- **窗口无法绝对定位**：纯 Wayland 下覆盖层按输出全屏，而不是按坐标摆放；存在 XWayland（`DISPLAY` 可用）时，应用会优先使用 winit 的 X11 后端以精确定位多显示器覆盖层。
- **贴图无法通用置顶**：贴图窗口会降级为普通无边框窗口。
- **依赖桌面门户**：截屏通过 `xdg-desktop-portal` + PipeWire。需要安装门户及其桌面后端：
  - GNOME：`xdg-desktop-portal-gnome`
  - KDE：`xdg-desktop-portal-kde`
  - wlroots（Sway / Hyprland 等）：`xdg-desktop-portal-wlr`
  - 其他合成器需确认对应的 `xdg-desktop-portal-*` 后端
- **授权窗口**：门户只允许当前焦点应用弹出授权对话框。托盘进程没有窗口时首次截图可能失败，应用会显示授权引导窗口，点击「授权并截屏」后再发起抓屏。
- **单次门户抓屏**：Linux Wayland 下优先一次抓取整个虚拟桌面再按显示器切分，避免逐屏触发多次门户请求；若切分结果不完整，会回退为逐显示器抓取。
- **托盘支持取决于桌面**：GNOME 默认不显示 StatusNotifierItem 托盘，需要安装「AppIndicator and KStatusNotifierItem Support」扩展；也可以用 `screenscan --preferences` 打开设置。

### X11

- 剪贴板内容由持有它的进程负责。进程退出后，如果没有剪贴板管理器（如 clipman、copyq），粘贴目标可能拿不到图片。常驻托盘模式可以避免该问题；一次性模式退出前会额外等待 500 ms。
- 全局快捷键通过 `XGrabKey` 注册，可能与其他应用冲突；注册失败只会记录日志，仍可通过托盘截屏。

### macOS

- 需要「屏幕录制」权限（系统设置 → 隐私与安全性 → 屏幕录制）。**首次授权后建议从 `.app` 启动**，裸二进制运行的授权可能不稳定；授权后通常需要重启应用。
- 发布包使用 ad-hoc 签名，未做公证，首次启动可能被 Gatekeeper 拦截（见[预编译包](#预编译包)）。
- 截屏使用 ScreenCaptureKit，最低支持 macOS 12.3。

### Windows

- 安装包按当前用户安装，不写入系统级目录，也不请求管理员权限。
- 其他应用可能把置顶窗口从置顶队列中「摘掉」；贴图窗口在失去焦点时会重新断言置顶。

### 功能状态

- 托盘菜单中的「贴图」入口、截图历史存储、检查更新目前仍是占位或预留实现。
- 界面语言切换目前作用于托盘菜单、首选项和授权窗口；截图工具栏与提示文本暂为中文。

## 架构

### 项目结构

```text
src/
├── main.rs            # 入口：日志、字体、事件循环、CLI 参数
├── app.rs             # App 状态容器、ApplicationHandler、UserEvent 分发
├── graphics.rs        # 每窗口独立的 glutin OpenGL 上下文与 egui 渲染封装
├── capture/
│   ├── mod.rs         # xcap 封装：枚举显示器、抓屏、Wayland 切分
│   └── portal.rs      # 通过 xdg-desktop-portal 一次性抓取整个虚拟桌面
├── overlay/
│   ├── mod.rs         # 覆盖层状态机、输入处理、复制/保存/贴图/扫描动作
│   ├── selection.rs   # 选区矩形、8 向命中测试、缩放与平移约束
│   └── render.rs      # 冻结截图、蒙层挖空、边框、控制点、尺寸标签
├── annotate/
│   ├── mod.rs         # 笔画列表、撤销/重做、按笔画擦除
│   ├── stroke.rs      # Tool / Stroke 数据模型
│   └── render.rs      # egui 实时预览 + tiny-skia 导出光栅化
├── toolbar/mod.rs     # egui 浮动工具栏
├── actions/
│   ├── mod.rs         # 动作结果类型与分发
│   ├── copy.rs        # 图片写入剪贴板
│   ├── save.rs        # 系统保存对话框 + 图片编码
│   └── scan.rs        # rxing 解码与预处理
├── pin.rs             # 贴图悬浮窗
├── tray.rs            # 系统托盘与菜单
├── settings.rs        # 首选项窗口（六个标签页）
├── permission.rs      # Wayland 截屏授权引导窗口
├── config.rs          # config.ini 读写、迁移、文件名模板
├── i18n.rs            # 托盘 / 设置 / 授权窗口的中英文字典
├── font.rs            # 系统字体扫描、自定义字体、CJK 字体回退
├── logging.rs         # 终端 + 按天滚动文件日志
├── autostart.rs       # 三平台开机启动
├── history.rs         # 截图历史存储接口（预留）
├── platform/mod.rs    # Wayland 检测、XWayland 后端选择、指针位置查询
└── util/
    ├── image.rs       # 按物理像素裁剪
    └── path.rs        # 主目录、~ 展开、路径展示
```

### 设计要点

1. **冻结截图，而非透明窗口**
   覆盖层先显示一张抓好的截图，再在其上画半透明蒙层；选区内部通过带 UV 的图片绘制「贴回」原图。这样不需要透明窗口，规避了 macOS 上 OpenGL 透明窗口变黑、Wayland 无法摆放窗口等问题。截图只在触发时抓一次，后续每帧只绘制几个矩形和一张纹理。

2. **每显示器一个覆盖层**
   多显示器和混合 DPI 下，跨屏单一窗口容易在定位与缩放上出错。应用为每块显示器创建独立窗口：X11/XWayland 使用绝对坐标与尺寸，纯 Wayland 使用「按输出全屏」。鼠标进入某块屏幕后，该屏幕变为活动层并显示选区与工具栏。

3. **物理像素与逻辑点分离**
   显示器截图与选区矩形使用物理像素；笔画点使用逻辑点，避免 UI 缩放导致线宽异常。导出时统一乘以窗口 `scale_factor` 并减去裁剪原点，保证预览与保存结果一致。

4. **笔画列表 + 按笔画擦除**
   标注模型是 `Vec<Stroke>`，每笔记录工具、颜色、线宽和点序列。橡皮擦不是擦像素，而是删除与擦除路径相交的整条笔画；撤销栈记录「新增笔画」或「一批被擦除的笔画」，因此撤销 / 重做无需维护像素快照。导出时用 tiny-skia 把笔画光栅化到裁剪图上。

5. **后台线程 + 事件回传**
   抓屏、条码识别、字体扫描、托盘图标与配置目录选择都可能阻塞。它们运行在后台线程，结果通过 winit 的 `EventLoopProxy<UserEvent>` 送回主线程；窗口创建和渲染始终在主线程完成。

6. **Wayland 单次门户抓屏**
   逐个显示器调用门户会让 GNOME 每块屏幕闪一次。应用改为一次请求整个虚拟桌面，再按显示器矩形切分，既减少闪烁，也保证所有屏幕来自同一时刻；失败时回退到逐屏抓取。

### 技术栈

| 领域 | 依赖 |
| --- | --- |
| 屏幕捕获 | `xcap`（DXGI / ScreenCaptureKit / X11 / portal）、`zbus`（Wayland 门户） |
| 窗口与事件 | `winit` |
| UI 与渲染 | `egui`、`egui-winit`、`egui_glow`、`glutin`、`glow` |
| 标注光栅化 | `tiny-skia`、`image` |
| 剪贴板 | `arboard` |
| 文件对话框 | `rfd`（Linux 使用 `xdg-portal` 后端） |
| 条码识别 | `rxing` |
| 系统托盘 | `tray-icon`（Linux 使用 `ksni`，不依赖 GTK） |
| 全局快捷键 | `global-hotkey` |
| 字体 | `ttf-parser` + 系统字体扫描 |
| 配置与日志 | 自研 INI 解析、`chrono`、`log`、`env_logger` |

## 开发与发布

```bash
# 格式化检查
cargo fmt --all -- --check

# 静态检查
cargo clippy --all-targets

# 测试
cargo test --locked

# 发布构建
cargo build --release --locked
```

调试时建议开启日志：

```bash
RUST_LOG=debug cargo run -- --preferences
```

发布流程由 `.github/workflows/release.yml` 驱动：

- 推送 `v*` 标签（例如 `v0.1.0`）会构建 Linux（x86_64 / aarch64）、Windows（x86_64 / arm64）、macOS（arm64 / x86_64）产物，并创建 GitHub Release。
- 手动触发工作流会构建开发版本 `<Cargo.toml 版本>.dev<短 SHA>`，仅上传构建产物，不创建 Release。
- 标签名包含 `-`（例如 `v0.2.0-beta.1`）时发布为预发布版本。

Linux 构建需要 `libpipewire-0.3-dev`、`libgbm-dev`、`libdrm-dev`、`libegl1-mesa-dev`、`libclang-dev`、Wayland / XCB 开发库；打包还需要 `libarchive-tools`、`zstd`、`librsvg2-bin` 和 `nfpm`。具体列表见工作流与 [`packaging/linux/package.sh`](packaging/linux/package.sh)。

## 故障排查

| 现象 | 排查方向 |
| --- | --- |
| GNOME 下看不到托盘图标 | 安装 AppIndicator / KStatusNotifierItem 扩展，或用 `screenscan --preferences` 打开设置 |
| Wayland 下 `Ctrl+Shift+S` 无反应 | Wayland 不支持全局快捷键，改用桌面快捷键绑定 `screenscan -c` |
| Wayland 下首次截图无响应 / 不弹授权框 | 确认 `xdg-desktop-portal` 与对应后端已安装；在授权引导窗口中点击「授权并截屏」；wlroots 安装 `xdg-desktop-portal-wlr` |
| Wayland 下贴图不置顶 | 合成器限制，贴图窗口会降级为普通窗口 |
| macOS 点击截屏无反应 / 画面全黑 | 在「系统设置 → 隐私与安全性 → 屏幕录制」中允许 ScreenScan，并从 `.app` 启动后重启应用 |
| macOS 提示应用已损坏或无法打开 | 发布包未公证，按[预编译包](#预编译包)中的方式放行 |
| X11 下复制成功但粘贴不到图片 | 安装剪贴板管理器（clipman / copyq），或使用常驻托盘模式 |
| 中文显示为方框 | 安装中文字体（如 Noto Sans CJK），或在首选项中指定界面字体 |
| 保存文件名不是预期格式 | 检查 `output.filename_format`，时间占位符必须用 `$...$` 包裹 |
| Linux 编译报错找不到 `libpipewire` / `libclang` | 安装[从源码构建](#从源码构建)中列出的系统依赖 |
| 需要提交问题日志 | 首选项「输出 → 打开日志目录」，附上对应日期的日志文件 |

## 贡献

欢迎提交 Issue 和 Pull Request：

1. Fork 仓库并从 `main` 创建分支，例如 `feat/xxx`、`fix/xxx`。
2. 保持提交信息清晰，建议使用 `feat:`、`fix:`、`perf:`、`docs:`、`refactor:` 等前缀（可参考现有提交历史）。
3. 提交前运行 `cargo fmt --all`、`cargo clippy --all-targets` 和 `cargo test --locked`。
4. 在 PR 中说明改动动机、实现方式和测试过的平台。

涉及界面文案的修改请注意：托盘、首选项、授权窗口的文本通过 `src/i18n.rs` 管理；截图工具栏目前仍为中文硬编码。

## 许可证

本项目基于 [MIT License](LICENSE) 开源，版权归 **Spinach5** 所有。

你可以自由使用、修改、分发本项目，包括商业用途，但需保留原始版权声明与许可证声明。软件按「原样」提供，不附带任何明示或暗示的担保。

## 致谢

- [xcap](https://github.com/nashaofu/xcap) — 跨平台屏幕捕获
- [winit](https://github.com/rust-windowing/winit) — 窗口与事件循环
- [egui](https://github.com/emilk/egui) — 即时模式 UI 与渲染
- [tiny-skia](https://github.com/RazrFalcon/tiny-skia) — 标注光栅化
- [rxing](https://github.com/rxing-core/rxing) — 条码 / 二维码识别
- [arboard](https://github.com/1Password/arboard) — 剪贴板
- [rfd](https://github.com/PolyMeilex/rfd) — 原生文件对话框
- [tray-icon](https://github.com/tauri-apps/tray-icon)、[ksni](https://github.com/iovxw/ksni) — 系统托盘
- [global-hotkey](https://github.com/tauri-apps/global-hotkey) — 全局快捷键

以及 Snipaste、Flameshot 等优秀截图工具在交互设计上的启发。
