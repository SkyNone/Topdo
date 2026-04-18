<div align="center">

# Topdo

### 把任务放在你眼前，而不是藏在窗口背后

macOS 桌面悬浮任务应用，支持本地离线与飞书同步  
Built with **Tauri 2 + Vue 3**

![Platform](https://img.shields.io/badge/platform-macOS-111111?style=flat-square)
![Stack](https://img.shields.io/badge/stack-Tauri%202%20%2B%20Vue%203-0ea5e9?style=flat-square)
![Arch](https://img.shields.io/badge/arch-Universal%20(Apple%20Silicon%20%2B%20Intel)-10b981?style=flat-square)

</div>

---

## 产品预览

![Topdo 功能说明](docs/images/功能说明.png)
![Topdo 产品介绍图](docs/images/介绍图.png)

---

## 一句话价值

Topdo 是一个“**常驻、低打扰、键盘优先**”的任务悬浮窗：  
你可以在任意工作流中快速记录、推进状态，并在需要时同步到飞书多维表格。

---

## 为什么你会喜欢 Topdo

- **始终可见**：悬浮窗常驻，不用来回切应用
- **执行导向**：待办 / 进行中 / 已完成，状态清晰
- **双模式**：本地开箱即用；飞书模式支持协同
- **键盘效率**：从新建到切换筛选，几乎不离开键盘
- **发布友好**：支持 Universal macOS 安装包

---

## 30 秒上手

### 1) 安装应用

下载发布页中的 `Topdo_*.dmg`，拖拽 `Topdo.app` 到 `Applications`。

### 2) 选择模式

- **本地模式**：立即开始，无需配置
- **飞书同步模式**：在设置中填写凭证并连接表格

### 3) 开始管理任务

按 `⌘N` 新建任务，点击状态圈推进任务，按 `⌘1~⌘4` 快速切换筛选。

---

## 核心能力

| 能力 | 说明 |
|---|---|
| 本地模式 | SQLite 持久化，离线可用 |
| 飞书同步 | 对接飞书多维表格，适合跨端协作 |
| 三态流转 | 待办 -> 进行中 -> 已完成 |
| 快捷键体系 | 新建、筛选、设置、快捷键面板 |
| 桌面体验 | 置顶、迷你模式、托盘驻留 |

---

## 快捷键速查

### 全局

- `⌘N`：新建任务
- `⌘,`：打开设置
- `⌘K`：快捷键面板
- `⌘⇧L`：浅色 / 深色切换
- `Esc`：关闭当前弹层

### 筛选栏

- `⌘1`：待办
- `⌘2`：进行中
- `⌘3`：已完成
- `⌘4`：全部

### 列表操作

- `↑ / ↓`：移动焦点
- `Enter`：展开/收起详情
- `⌘Enter`：切换任务状态

---

## 飞书同步配置

在设置页填写以下字段：

- `App ID`
- `App Secret`
- `App Token`（多维表格链接 `/base/` 后 token）
- `Table ID`（链接中 `table=` 参数）

建议确认：

- 应用已开通 bitable 相关权限
- 应用与目标多维表格在同一租户
- 应用版本已发布可用

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
pnpm dev
pnpm build
pnpm tauri dev
cd src-tauri && cargo check
```

---

## 发布（Universal macOS）

```bash
pnpm release:mac
```

产物路径：

- `.app`：`src-tauri/target/universal-apple-darwin/release/bundle/macos/Topdo.app`
- `.dmg`：`src-tauri/target/universal-apple-darwin/release/bundle/dmg/Topdo_1.0.0_universal.dmg`

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
- [ ] 更完善的飞书同步状态提示
- [ ] 搜索与标签体系
- [ ] 正式签名与公证发布

---

## License

MIT（建议）

---

<div align="center">

如果这个项目对你有帮助，欢迎 Star ⭐

</div>
