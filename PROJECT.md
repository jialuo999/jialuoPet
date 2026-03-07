

# Niri桌面宠物项目大纲（重构版）

## 一、项目定位与约束

| 维度 | 定义 |
|------|------|
| **核心目标** | 在Niri（Wayland）环境下实现一个始终置顶、透明背景、低资源占用的桌面宠物 |
| **主要功能** | 动画播放、点击/拖拽交互、状态面板、设置和缩放 |
| **非目标** | 不追求跨桌面环境兼容，不提前实现交互/商店等扩展功能 |
| **质量底线** | 内存占用<100MB，CPU占用<5%，24小时运行不泄漏 |
| **开发节奏** | 三步走：原型验证→稳定运行→功能扩展 |

---

## 二、技术栈（经版本验证）

```toml
# Cargo.toml - 依赖锁定
[package]
name = "niri-pet"
version = "0.1.0"
edition = "2021"

[dependencies]
gtk = { version = "0.9.6", features = ["v4_14"] }
gtk4-layer-shell = "0.5.0"  # 对应GTK4 v0.9.x
glib = "0.9.6"
gio = "0.9.6"
gdk-pixbuf = "0.9.6"
anyhow = "1.0.97"      # 错误处理
once_cell = "1.21.3"   # 全局配置
dirs = "6.0.0"         # XDG目录规范
```

**系统依赖安装命令：**
```bash
sudo pacman -S rustup gtk4 gtk4-layer-shell
rustup default stable
```

> **版本锁定原则**：所有GTK生态库使用0.9.x系列，确保ABI兼容。gtk4-layer-shell 0.5.0是适配GTK4 0.9的最新版。

---

## 三、文件结构（分阶段演进）

### 阶段一：单文件验证（当前，<200行）

```
niri-pet/
├── Cargo.toml
├── src/
│   └── main.rs          # 包含：应用入口、窗口配置、动画循环、资源加载
└── assets/
    └── idle/            # 默认动画：待机
        ├── 0001.png     # 必须4位数字，从0001开始
        ├── 0002.png
        └── ...
```

**阶段一硬性约束：**
- `main.rs` 超过150行必须启动阶段二
- 所有`unwrap()`必须附带注释说明为何不会panic
- 资源路径使用`dirs::data_dir()`，禁止硬编码绝对路径

### 阶段二：模块化拆分（main.rs>150行时触发）

```
src/
├── main.rs              # 仅保留gtk应用生命周期
├── app.rs               # PetApp结构体：状态管理
├── window.rs            # Layer Shell窗口创建与配置
├── animator.rs          # 帧动画引擎（预加载、帧率控制）
├── loader.rs            # 资源加载与热重载支持
├── config.rs            # 配置管理（位置、动画速度等）
├── settings/            # 设置模块：持久化存储、设置面板、缩放
│   ├── model.rs         # 设置数据模型
│   ├── panel.rs         # 设置UI（位置记忆、缩放滑块）
│   ├── storage.rs       # TOML持久化
│   └── mod.rs
└── interaction/         # 交互层：拖拽、触摸、菜单
    └── mod.rs
```

### 阶段三：功能扩展（稳定运行48小时后）

```
src/
├── main.rs
├── app.rs
├── window.rs
├── animator.rs
├── loader.rs
├── config.rs
├── interaction/         # 新增：点击/拖拽交互
│   ├── mod.rs
│   └── handler.rs
└── ipc.rs               # 新增：外部控制接口（供后续商店/配置工具调用）
```

---

## 四、核心实现规范

### 4.1 窗口配置（Niri专用）

```rust
// 必须显式设置的Layer Shell属性
layer.set_layer(Layer::Overlay);           // 层级：Overlay（最顶层）
layer.set_exclusive_zone(-1);              // 不保留屏幕空间
layer.set_keyboard_mode(KeyboardMode::None); // 不抢占键盘焦点
layer.set_anchor(Edge::Right | Edge::Bottom, true); // 默认位置：右下

// Niri特定：避开顶部工作区指示器（通常高30-40px）
layer.set_margin(Edge::Top, 50);
layer.set_margin(Edge::Right, 20);
```

### 4.2 动画引擎规范

| 项目 | 规范 | 理由 |
|------|------|------|
| 帧率控制 | 使用`gtk::glib::timeout_add_local`，禁止自建线程 | GTK非线程安全，Wayland上下文必须在主线程 |
| 资源加载 | 启动时预加载全部帧到`Vec<Pixbuf>` | 避免运行时IO阻塞，实现零分配切换 |
| 内存管理 | 使用`Pixbuf`的引用计数，不手动`unref` | 交给Rust Drop trait，避免UAF |
| 热重载 | 监听`SIGUSR1`信号，重新扫描assets目录 | 不重启程序更换皮肤 |

### 4.3 错误处理策略

```rust
// 三级错误处理
pub enum PetError {
    Fatal(anyhow::Error),      // 资源缺失、GTK初始化失败 → 立即退出
    Recoverable(String),       // 单帧加载失败 → 跳过该帧，打印日志
    Warning(String),           // 配置解析错误 → 使用默认值，通知用户
}

// 所有资源加载必须返回Result，禁止unwrap()
let frames = loader::load_animation("idle")
    .context("加载待机动画失败")?;  // Fatal：没有默认动画程序无法运行
```

### 4.4 缩放功能规范

| 项目 | 规范 | 理由 |
|------|------|------|
| 缩放范围 | 50% ~ 200%，步进 5% | 保证最小可用性，避免过小/过大 |
| 默认值 | 100% (DEFAULT_PIXEL_SIZE = 256px) | 与原始素材1:1对应 |
| 实时预览 | 滑块拖动时立即调用 `image.set_pixel_size()` | 用户实时看到效果 |
| 持久化 | 保存到 `settings/user_settings.toml` | `remember_position`、`window_position`、`scale_factor`、`auto_close_panels_on_outside_click` |
| 交互适配 | 所有触摸区域用 `widget_size / pixbuf_size` 比率映射 | 缩放不影响命中精度 |
| 输入区域 | 每帧重算 alpha 通道输入区域 | 缩放后 alpha 区域自动缩放 |

---

## 五、资源与配置规范

### 5.1 目录结构（运行时）

```
~/.local/share/niri-pet/          # dirs::data_dir()
├── animations/
│   ├── idle/                       # 默认状态
│   │   ├── 0001.png
│   │   └── ...
│   └── click/                      # 点击反馈（预留）
│       └── ...
└── config.toml                     # 位置、帧率等配置

~/.config/niri-pet/                # 用户设置目录
└── user_settings.toml              # 用户偏好：位置、缩放因子
```

### 5.2 settings/user_settings.toml 格式

```toml
remember_position = true

[window_position]
left = 1137
top = 356

scale_factor = 1.0  # 范围 0.5 ~ 2.0，默认 1.0（100%）

# false: 手动关闭面板
# true: 点击宠物空白处自动关闭状态/投喂面板
auto_close_panels_on_outside_click = false
```

### 5.3 PNG文件规范

- **命名**：`0001.png` ~ `9999.png`，必须连续，缺失则停止加载
- **格式**：RGBA 8bit，透明通道必须存在
- **尺寸**：建议256x256，所有帧尺寸必须一致
- **颜色配置**：禁止CMYK，必须为sRGB

---

## 六、验证清单（每阶段必须通过）

### 阶段一出口条件
- [ ] 在Niri下正常显示，始终置顶
- [ ] 透明背景正常，无黑边/白边
- [ ] 连续运行1小时，内存增长<10MB
- [ ] 删除assets目录后程序优雅退出并提示错误

### 阶段二出口条件
- [ ] 代码覆盖率>60%（单元测试）
- [ ] 支持运行时资源重载
- [ ] 多显示器切换时宠物保持在当前活动输出
- [ ] **设置面板**：位置记忆和缩放功能正常
  - [ ] 缩放范围 50%~200% 可正常操作
  - [ ] 缩放实时预览正确
  - [ ] 恢复默认按钮恢复至 100%
  - [ ] 缩放因子正确持久化到 TOML
  - [ ] 重启程序能恢复之前的缩放设置
  - [ ] 缩放时交互区域（触摸/拖拽）正确映射
- [ ] 取消/退出设置面板时，缩放预览回滚至已保存值

### 阶段三入口条件
- [ ] 连续运行72小时无内存泄漏（valgrind验证）
- [ ] CPU平均占用<3%（`ps`采样）

---

## 七、风险与规避

| 风险 | 影响 | 规避方案 |
|------|------|----------|
| GTK4 Layer Shell在Niri版本更新后行为变化 | 窗口无法置顶或崩溃 | 锁定gtk4-layer-shell版本，升级前在VM测试 |
| 高频动画导致Wayland连接断开 | 宠物消失，需重启 | 使用`gdk_frame_clock`同步刷新率，避免超过显示器刷新率 |
| 大尺寸PNG（>1MB/帧）导致OOM | 系统卡顿 | 启动时检查单帧尺寸，超过512x512拒绝加载并提示 |
| Niri切换输出时宠物位置异常 | 宠物跑到屏幕外或不可见区域 | 监听`window.state_flags`变化，自动重新锚定 |
| 缩放时UI快速抖动 | 用户体验差 | 使用`glib::idle_add_local`批量更新，避免逐帧刷新 |
| 缩放后交互坐标计算错误 | 点击无效或命中不准 | 使用 widget_size / pixbuf_size 比率映射，每帧刷新输入区域 |

---

## 八、开发命令速查

```bash
# 开发运行
cargo run

# 发布构建（优化大小）
cargo build --release

# 内存泄漏检测
valgrind --tool=memcheck --leak-check=full ./target/release/niri-pet

# 资源热重载测试
pkill -USR1 niri-pet

# 安装到本地
cargo install --path .
mkdir -p ~/.local/share/niri-pet/animations/idle
cp -r assets/idle/* ~/.local/share/niri-pet/animations/idle/
```

