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

按 `⌘N` 新建任务，点击状态圈推进任务，按 `⌘1~⌘4` 快速切换筛选，按 `⌘F` 快速搜索。

---

## 核心能力

| 能力 | 说明 |
|---|---|
| 本地模式 | SQLite 持久化，离线可用 |
| 飞书同步 | 对接飞书多维表格，适合跨端协作 |
| 三态流转 | 待办 -> 进行中 -> 已完成 |
| 快捷键体系 | 新建、搜索、筛选、设置、快捷键面板 |
| 桌面体验 | 置顶、迷你模式、托盘驻留 |
| 轻量编辑 | 双击任务标题直接编辑，详情区可改优先级与备注 |

---

## 快捷键速查

### 全局

- `⌘N`：新建任务
- `⌘F`：搜索任务
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
- 双击任务标题：直接编辑名称
- `Esc`：取消行内编辑

---

## 搜索与编辑体验

- `⌘F` 打开搜索浮层，可按任务名称和备注即时过滤
- 搜索关闭后会保留当前结果，并在顶部持续显示搜索词和结果数
- 双击任务标题可直接重命名，回车或失焦保存
- 任务详情区支持直接修改优先级与备注/收获

---

## 设置与反馈

- 设置页底部提供：
  - GitHub 项目主页
  - 用户反馈入口（GitHub Issues）
- 飞书同步设置中的模板和教程链接支持直接拉起默认浏览器
- 常见错误支持复制详情，便于排查同步问题

---

## 迷你模式

- 宠物开启时：缩小后进入猫咪 mini 形态
- 宠物关闭时：缩小后进入横向胶囊 mini，显示 `Topdo + 待办数`
- 迷你模式支持点击恢复，并保留拖动与位置记忆

---

## 2.0 宠物模式（阶段一）

- 新增两种窗口形态：`面板态` / `猫咪态`
- 默认形态切换快捷键：`⌥T`（可在设置页自定义）
- 猫咪等级随今日完成进度变化：Sleeping / Awake / Happy / Crowned
- 支持角标显示未完成数量、可选动画开关、位置记忆

> 说明：阶段一先上线猫咪态，不含顶部吸附灵动岛能力。

### 猫咪状态切换规则

- 今天完成第 `1` 个任务：`awake`
- 今天完成第 `3` 个任务：`happy`
- 当天全部任务都已完成：`crowned`
- 当天未完成任何任务：`sleeping`
- 判定口径：仅统计 `completed_at` 落在今天、且当前状态仍为“已完成”的任务
- 同一天内小猫保持当天已达成的最高状态，不会因为新增任务或回退任务立刻降级
- 次日首次打开应用后，今日进度会自动重置
- 瞬态动画：
  - 新增任务：`new_task`（0.5 秒）
  - 首次达到当天全部完成：`celebrate`（2 秒）

### 今日进度激励

- 飞书模式下，完成任务后小猫会立即更新状态，不必等待下一轮同步
- 小猫更偏向“正向反馈”，强调今天完成了多少，而不是总库存完成率
- 对于任务少的用户，完成 1 个任务也能立刻看到从 `sleeping` 到 `awake` 的变化

### 宠物素材规范（阶段一）

- 文件名固定：`sleeping.svg`、`awake.svg`、`happy.svg`、`crowned.svg`
- 资源策略：仅加载 SVG（不再回退 PNG），避免白底素材误渲染
- 推荐规格：透明背景、像素风主体居中
- 若素材缺失或加载失败，应用会自动降级为透明占位渲染，避免出现白色方块

---

## 唤起快捷键冲突处理

如果 `⌘⇧T` 与浏览器或输入法冲突，可在设置页「全局唤起快捷键」中修改。

- 推荐组合：`Cmd+Alt+T`、`Cmd+Shift+Y`
- 规则：必须包含至少一个修饰键（Cmd/Alt/Ctrl/Shift）
- 避免与应用内保留快捷键冲突：`⌘N`、`⌘K`、`⌘1~⌘4`、`⌘⇧R`、`⌘⇧L`

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

优先级映射规则：

- Topdo 本地文案：
  - `紧急`
  - `重要`
  - `普通`
- 飞书多维表格枚举：
  - `今日必做`
  - `本周完成`
  - `自由安排`

写入飞书时会自动映射，避免在多维表格中误创建新的优先级枚举值。

如果报错较长，设置页已支持“复制错误详情”，可直接贴给开发者排查。

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

> macOS 透明窗口依赖 `src-tauri/tauri.conf.json` 中的 `app.macOSPrivateApi: true`。  
> 这会启用 macOS private API，因此不适合 App Store 分发。

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
- `.dmg`：`src-tauri/target/universal-apple-darwin/release/bundle/dmg/Topdo_1.0.1_universal.dmg`

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
