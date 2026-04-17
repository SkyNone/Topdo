# Topdo

Topdo 是一个基于 **Tauri 2.0 + Vue 3** 的 macOS 桌面任务悬浮窗应用，聚焦“轻量、可见、快速执行”。

## 产品说明

### 产品定位

Topdo 面向希望在桌面侧边持续管理任务的个人用户与小团队。应用常驻、低打扰，支持本地模式和飞书同步模式。

### 核心能力

- 本地离线任务管理（SQLite）
- 飞书多维表格同步（可选）
- 快捷键体系（如 `⌘N` 新建、`⌘1~⌘4` 筛选、`⌘K` 快捷键面板）
- 迷你模式、置顶、托盘驻留

### 适用场景

- 个人日常任务管理（待办 / 进行中 / 已完成）
- 跨设备协作（飞书多维表格同步）
- 高频切换场景下的快速记录与状态推进

## 产品截图

图片目录约定：`docs/images/`

> 你提供图片后按以下命名放入即可，README 会直接渲染：
> `overview.png`、`welcome.png`、`task-list.png`、`settings.png`、`mini-mode.png`

<!--
示例（有图后取消注释）：
![主界面总览](docs/images/overview.png)
![欢迎页](docs/images/welcome.png)
![任务列表](docs/images/task-list.png)
![设置页](docs/images/settings.png)
![迷你模式](docs/images/mini-mode.png)
-->

当前截图状态：待补充（你提供素材后我可帮你一次性接入）。

## 快速开始

## 环境要求

- macOS 10.15+
- Node.js 18+
- pnpm 10+
- Rust stable
- Xcode Command Line Tools

```bash
xcode-select --install
corepack enable
corepack prepare pnpm@10.33.0 --activate
```

## 开发运行

```bash
cd /Users/bytedance/Documents/X_projects/task-float
pnpm install
pnpm tauri dev
```

## 基本使用

1. 首次启动选择模式：
   - 本地模式：开箱即用
   - 飞书模式：进入设置页配置
2. 顶部 `+` 按钮或 `⌘N` 新建任务
3. 点击任务状态圈切换状态
4. 点击任务展开备注，自动保存
5. `⌘K` 查看完整快捷键面板

## 飞书配置说明

在设置页填写：

- `App ID`：飞书开放平台应用 ID
- `App Secret`：飞书开放平台应用密钥
- `App Token`：多维表格链接中 `/base/` 后的 token
- `Table ID`：多维表格链接中 `table=` 参数值

建议确认：

- 应用已开通 bitable 相关权限
- 应用与目标多维表格在同一租户
- 应用已发布可用版本

## 构建 macOS 安装包（Universal .app + .dmg）

### 1) 执行打包（推荐）

```bash
cd /Users/bytedance/Documents/X_projects/task-float
pnpm release:mac
```

> 该命令会自动构建 universal `.app + .dmg`（兼容 Apple Silicon 与 Intel）。
> 同时会在 DMG 根目录加入 `01_安装说明.txt`，用户安装时可直接看到。

### 2) 产物目录

- `.app`：
  `/Users/bytedance/Documents/X_projects/task-float/src-tauri/target/universal-apple-darwin/release/bundle/macos/Topdo.app`
- `.dmg`：
  `/Users/bytedance/Documents/X_projects/task-float/src-tauri/target/universal-apple-darwin/release/bundle/dmg/Topdo_1.0.0_universal.dmg`

### 3) DMG 安装引导（拖拽安装）

打包产出的 DMG 会使用标准安装窗口布局（App 图标 + Applications 文件夹图标）：

1. 双击打开 `Topdo_1.0.0_universal.dmg`
2. 将 `Topdo.app` 拖拽到 `Applications` 文件夹
3. 打开同目录的 `01_安装说明.txt`（建议先看）
4. 从“应用程序”中启动 Topdo

> 推荐对外分发 `universal` 包，兼容 Apple Silicon 与 Intel Mac。

### 4) 未签名应用首次打开（Gatekeeper）

方式 A（附加方式）：

1. Finder 中找到 `Topdo.app`
2. 右键 -> 打开
3. 再次确认“打开”

方式 B（推荐修复，直接在终端执行）：

```bash
xattr -cr /Applications/Topdo.app
```

可选兜底（仅在上一步后仍打不开时）：

```bash
codesign --force --deep --sign - /Applications/Topdo.app
open /Applications/Topdo.app
```

> 注意：未签名/未公证包在部分 macOS 上可能被拦截，这不代表安装包损坏。

## 常用命令

```bash
# 前端开发
pnpm dev

# Tauri 开发
pnpm tauri dev

# 前端构建
pnpm build

# Rust 检查
cd src-tauri && cargo check
```

## 图标资源

当前图标位于：

- `/Users/bytedance/Documents/X_projects/task-float/src-tauri/icons/icon.icns`
- `/Users/bytedance/Documents/X_projects/task-float/src-tauri/icons/icon.png`

替换图标示例：

```bash
cd /Users/bytedance/Documents/X_projects/task-float
cargo tauri icon -o src-tauri/icons path/to/your-icon.svg
```
