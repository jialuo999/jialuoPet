# niri-pet 架构说明

本文档描述当前代码实现下的系统架构、核心数据流与事件流，帮助快速理解项目如何运行。

## 1. 项目目标与运行环境

- 目标：在 Niri（Wayland）桌面中运行一个透明、置顶、可交互的桌宠。
- UI 框架：GTK4。
- 窗口能力：`gtk4-layer-shell`，用于 Overlay 层窗口和锚点控制。
- 动画资源：PNG 帧序列（位于 `assets/body/...`）。

## 2. 目录与模块分层

### 2.1 代码分层

- **应用入口层**：`src/main.rs`
  - 启动 GTK 应用、创建窗口、接入交互与菜单、连接动画与状态面板。
- **配置层**：`src/config.rs`
  - 维护默认常量、读取 `config.toml`、参数校验（sanitize）、配置热更新监听。
- **动画域**：`src/animation/*`
  - `coordinator.rs`：统一调度动画状态与帧推进。
  - `requests.rs`：通过原子变量传递动画请求（drag/pinch/touch/shutdown）。
  - `player/*`：各类动画播放器（startup/default idle/drag/pinch/touch/shutdown）。
  - `assets/*`：动画资源路径与帧收集逻辑。
- **交互层**：
  - `src/drag.rs`：长按拖拽、捏捏区域判断、窗口跟随与拖拽动画触发。
  - `src/input_region.rs`：输入区域裁剪、触摸头/身体区域判定、右键菜单。
- **状态展示层**：`src/stats_panel.rs`
  - 仅负责 GTK 可视化面板渲染（`StatsPanel`）。
- **状态计算层**：`src/stats/*`
  - `model.rs`：纯数据结构与纯计算函数（`PetStats`、`PetMode`、模式判断与等级公式）。
  - `service.rs`：带副作用的状态服务（衰减、投喂、互动、升级、配置上限应用）。
  - `mod.rs`：状态模块统一导出。

### 2.2 资源分层

- `assets/body/...` 按动作和状态组织（如 `StartUP`、`Default`、`Pinch`、`Touch_Head`、`Shutdown` 等）。
- `config.toml` 提供资源根目录和子路径映射，支持替换资源目录布局。

## 3. 启动流程（高层）

1. `main()` 初始化 `Application` 并进入 `build_ui()`。
2. `build_ui()` 创建透明 Layer-Shell 窗口并设置默认锚点（右下）。
3. 加载面板配置并创建 `PetStatsService`（包含初始值和面板上限）。
4. 调用 `animation::load_carousel_images()`：
   - 构建各类 player；
   - 设置首帧；
   - 启动定时循环（`CAROUSEL_INTERVAL_MS`，当前 130ms）。
5. 绑定交互：输入探针、长按拖拽、头/身体点击区域、右键菜单。
6. 创建并连接 `StatsPanel`，并启动配置文件热更新监听。

## 4. 动画系统设计

### 4.1 请求机制（弱耦合）

交互层不会直接操作 player，而是写入 `animation/requests.rs` 中的原子请求位：

- Drag：start / loop / end
- Pinch：start / loop / end
- Touch：head / body
- Shutdown：request

动画调度器每个 tick 使用 `consume_requests()` 一次性消费请求，避免 UI 回调和动画状态机强耦合。

同时每个 tick 会先调用 `stats_service.on_tick(CAROUSEL_INTERVAL_MS as f64 / 1000.0)`，统一推进状态衰减。

### 4.2 调度优先级

在 `coordinator` 中，事件分发优先级为：

1. `shutdown`
2. `drag_raise`
3. `pinch`
4. `touch`

帧推进优先级为：

1. `shutdown`
2. `drag_raise`
3. `pinch`
4. `touch`
5. `startup`
6. `default_idle`

说明：`startup` 仅初始化时活跃，播完后自动回落到 `default_idle`。

### 4.3 模式驱动资源切换

- `PetStatsService::cal_mode()` 代理到 `stats/model.rs` 中的纯函数计算 `PetMode`（Happy / Nomal / PoorCondition / Ill）。
- `coordinator::maybe_update_mode()` 检测模式变化并触发 player `reload_for_mode()`。
- 这样动画资源会跟随状态模式切换，而不需要重建 UI。

## 5. 输入与交互系统

### 5.1 输入区域裁剪

`setup_image_input_region()` 会根据当前帧 alpha 区域设置窗口可点击区域：

- 透明区域可“穿透”到下层窗口；
- 仅宠物图像实体区域响应点击；
- 每帧切换时同步刷新输入区域，确保点击命中准确。

### 5.2 拖拽与捏捏

- 左键按下进入计时；超过 `DRAG_LONG_PRESS_MS`（默认 450ms）进入长按语义。
- 在指定区域且未明显移动时触发 pinch。
- 进入拖拽后切换窗口锚点到左上（绝对 margin 驱动），并持续发送 drag loop 请求。
- 松开时发送 drag end / pinch end。

### 5.3 触摸区域

- 在头部矩形区域触发 `request_touch_head_animation()`。
- 在身体矩形区域触发 `request_touch_body_animation()`。
- 不同 `PetMode` 使用不同矩形参数，匹配不同体态资源。

## 6. 状态面板与配置热更新

### 6.1 状态面板

- `StatsPanel` 通过 `Popover` 显示体力、饱腹、口渴、心情、健康、好感、经验、等级与模式。
- `StatsPanel` 只读 `stats_service.get_stats()` 与 `stats_service.cal_mode()`，不持有业务状态逻辑。
- 右键菜单中的“面板”按钮用于显示/隐藏。

### 6.2 配置热更新

`config.toml` 变更后流程：

1. `start_panel_config_watcher()` 监听配置文件变更；
2. 通知主线程 channel；
3. 主线程定时轮询并执行：
   - `load_panel_debug_config()`
   - `stats_service.apply_panel_config(...)`
   - `stats_panel.refresh()`

这保证了面板参数可在运行时无重启生效；其中 `basic_stat_max / experience_max / level_max` 直接影响面板显示上限，`default_*` 影响当前状态初始值/替换值。

## 7. 关键设计取舍

- **事件与播放解耦**：请求位 + tick 消费，降低回调复杂度。
- **计算与渲染解耦**：`stats/model.rs` 保持纯计算，`stats_panel.rs` 仅渲染，便于测试与维护。
- **单线程 UI 安全**：GTK 相关操作在主线程，避免线程访问 UI 风险。
- **资源路径可配置**：动画目录通过 `config.toml` 管理，便于换皮/重组资源。
- **输入区域跟帧同步**：提高交互命中精度，但会增加每帧 region 更新成本。

## 8. 典型扩展点

- 新增动作：
  1. 在 `player/` 增加对应 player；
  2. 在 `requests.rs` 增加请求位；
  3. 在 `coordinator.rs` 增加分发与优先级规则；
  4. 在 `assets/` 增加资源收集逻辑与配置项。
- 新增玩法逻辑：在 `PetStatsService` 中扩展 `on_tick/on_feed/on_interact` 并让动画/面板消费。
- 配置化交互区域：可将头/身体/捏捏矩形迁移到 `config.toml`，减少硬编码常量。

## 9. 当前架构一句话总结

该项目采用“GTK 主线程 + 定时调度器 + 原子请求队列 + 多播放器状态机”的结构，在保持交互响应的同时，实现了可配置、可扩展的桌宠动画系统。
