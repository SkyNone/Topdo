<div align="center">

# Topdo

### 把任务、习惯和提醒放在你眼前

macOS 桌面悬浮效率工具，支持本地离线、飞书同步、习惯打卡、截止提醒和轻量任务编辑。
Built with **Tauri 2 + Vue 3**

![Platform](https://img.shields.io/badge/platform-macOS-111111?style=flat-square)
![Version](https://img.shields.io/badge/version-v2.0.1-2563eb?style=flat-square)
![Arch](https://img.shields.io/badge/arch-Universal%20(Apple%20Silicon%20%2B%20Intel)-10b981?style=flat-square)

</div>

---

## 下载

- [下载 Topdo v2.0.1 macOS Universal DMG](https://github.com/SkyNone/Topdo/releases/download/v2.0.1/Topdo_2.0.1_universal.dmg)
- [查看 v2.0.1 Release](https://github.com/SkyNone/Topdo/releases/tag/v2.0.1)

下载 `.dmg` 后，将 `Topdo.app` 拖到 `Applications` 即可。

> 当前安装包未做 Apple notarization。首次打开若被 macOS 拦截，请参考下方「安装排障」。

---

## 一句话价值

Topdo 是一个“**常驻、低打扰、键盘优先**”的桌面任务工具：
你可以快速记录任务、推进状态、设置提醒、同步飞书，也可以用习惯模块追踪每天的固定动作。

---

## Topdo v2.0.1 当前能力

| 能力 | 说明 |
|---|---|
| 本地模式 | SQLite 本地持久化，离线可用，开箱即用 |
| 飞书同步 | 对接飞书多维表格，支持任务字段同步与优先级映射 |
| 任务三态 | 待办 / 进行中 / 已完成，适合执行推进 |
| 搜索过滤 | `⌘F` 按任务名称和备注搜索，顶部保留搜索状态 |
| 轻量编辑 | 双击标题改名，详情页可改优先级、截止时间、提醒、重复、子任务和备注 |
| 习惯模块 | 支持创建习惯、打卡、连续天数和习惯提醒 |
| 截止提醒 | 支持 macOS 系统通知，并提供应用内 toast 兜底提醒 |
| 数据导出 | 支持 JSON / CSV / Markdown 导出 |
| 桌面体验 | 置顶、迷你模式、托盘驻留、关闭宠物后的横向胶囊 mini |
| 宠物模式 | 猫咪状态、角标、动画反馈和位置记忆 |
| Universal 安装包 | 同时支持 Apple Silicon 和 Intel Mac |

---

## v2.0 主要新增

- **习惯模块**：新增任务 / 习惯双模式，可创建习惯并完成每日打卡。
- **提醒体系**：任务截止提醒和习惯提醒都会走 macOS 系统通知；Topdo 前台时还会显示应用内 toast，避免系统通知被折叠后看不见。
- **任务详情升级**：支持编辑截止日期与时间、重复规则、提醒、子任务、备注和优先级。
- **创建任务优化**：新建面板压缩为更适合小窗口的一屏交互，支持日期、时间、重复和提醒。
- **搜索与快捷键补全**：`⌘F` 搜索、`⌘K` 快捷键面板、`⌘J` 切换任务 / 习惯。
- **设置页收敛**：保留高频设置，支持反馈入口、模板教程外链、数据导出和版本检查。
- **飞书同步修正**：本地优先级会映射到飞书已有枚举，避免多维表格中被误创建新选项。

---

## 30 秒上手

### 1. 安装应用

[下载 `Topdo_2.0.1_universal.dmg`](https://github.com/SkyNone/Topdo/releases/download/v2.0.1/Topdo_2.0.1_universal.dmg)，拖拽 `Topdo.app` 到 `Applications`。

### 2. 选择模式

- **本地模式**：无需配置，立即开始。
- **飞书同步模式**：在设置页填写飞书应用凭证和多维表格信息。

### 3. 创建任务

按 `⌘N` 新建任务；点击状态圈推进任务；按 `⌘1~⌘4` 切换筛选；按 `⌘F` 搜索。

### 4. 开启习惯

在设置页开启「习惯模块」，点击标题下拉可在 `Topdo / 习惯` 之间切换。

---

## 快捷键

### 全局

- `⌘N`：新建任务
- `⌘F`：搜索任务
- `⌘,`：打开 / 关闭设置
- `⌘K`：快捷键面板
- `⌘J`：任务 / 习惯切换
- `⌘⇧L`：浅色 / 深色切换
- `Esc`：关闭当前弹层

### 筛选栏

- `⌘1`：待办
- `⌘2`：进行中
- `⌘3`：已完成
- `⌘4`：全部

### 列表操作

- `↑ / ↓`：移动焦点
- `Enter`：展开 / 收起详情
- `⌘Enter`：切换任务状态
- 双击任务标题：直接编辑名称
- `Esc`：取消行内编辑

---

## 任务与提醒

- 截止日期支持日期和时间；只选日期时默认按当天 `23:59` 到期。
- 任务提醒支持：到期时、提前 15 分钟、提前 1 小时、提前 1 天。
- 已完成任务不会继续统计或触发逾期提醒。
- 系统通知依赖 macOS 通知权限；如果系统通知被折叠，应用内 toast 仍会在 Topdo 前台时显示。

---

## 习惯模块

- 支持每天、工作日、自定义星期。
- 支持提醒时间、连续天数和今日打卡统计。
- 习惯提醒会在设置的时间触发；当天已打卡则不会重复提醒。
- 支持编辑、归档和删除习惯。

---

## 飞书同步

在设置页填写以下字段：

- `App ID`
- `App Secret`
- `App Token`（多维表格链接 `/base/` 后 token）
- `Table ID`（链接中 `table=` 参数）

优先级映射规则：

| Topdo | 飞书多维表格 |
|---|---|
| 紧急 | 今日必做 |
| 重要 | 本周完成 |
| 普通 | 自由安排 |

写入飞书时会自动映射，避免在多维表格中误创建新的优先级枚举值。

---

## 设置与反馈

- 设置页提供 GitHub 项目主页和用户反馈入口。
- 飞书模板和教程链接可直接用默认浏览器打开。
- 数据导出支持 JSON / CSV / Markdown。
- 点击设置页版本号可检查 GitHub 最新版本。

---

## 支持作者

Topdo 会保持核心功能免费可用。如果它真的帮你减少了任务混乱，欢迎 Star，或者[请作者喝杯咖啡](docs/SUPPORT.md)。

---

## 迷你模式与宠物模式

- 宠物开启时：缩小后进入猫咪 mini 形态。
- 宠物关闭时：缩小后进入横向胶囊 mini，显示 `Topdo + 待办数`。
- 迷你模式支持点击恢复，并保留拖动与位置记忆。
- 宠物模式支持角标、动画和猫咪状态反馈。

> 当前版本不包含顶部吸附灵动岛能力。

---

## 开发者指南

### 环境要求

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

### 本地开发

```bash
pnpm install
pnpm tauri dev
```

### 常用命令

```bash
pnpm build
cd src-tauri && cargo check
pnpm release:mac
```

---

## 发布产物

```bash
pnpm release:mac
```

产物路径：

- `.app`：`src-tauri/target/universal-apple-darwin/release/bundle/macos/Topdo.app`
- `.dmg`：`src-tauri/target/universal-apple-darwin/release/bundle/dmg/Topdo_2.0.1_universal.dmg`

---

## 安装排障（未公证版本）

部分 macOS 环境首次打开可能被系统拦截，可执行：

```bash
xattr -cr /Applications/Topdo.app
```

若仍失败：

```bash
codesign --force --deep --sign - /Applications/Topdo.app
open /Applications/Topdo.app
```

---

## 路线图

- [ ] 自动更新（Updater）
- [ ] 正式签名与公证发布
- [ ] 更完整的飞书字段映射
- [ ] 更细的搜索与标签体系

---

## License

MIT

---

<div align="center">

如果这个项目对你有帮助，欢迎 Star ⭐

</div>
