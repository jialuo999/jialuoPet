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
- **配置层**：`src/config/*`
  - `defaults.rs`：默认常量（应用 ID、动画间隔、拖拽参数等）。
  - `loader.rs`：读取 `config.toml` 并返回结构化配置。
  - `model.rs`：配置数据模型定义。
  - `watcher.rs`：配置热更新监听。
- **动画域**：`src/animation/*`
  - `coordinator.rs`：统一调度动画状态与帧推进。
  - `requests.rs`：通过原子变量传递动画请求（drag/pinch/touch/shutdown）。
  - `player/*`：各类动画播放器（startup/default idle/drag/pinch/touch/shutdown）。
  - `assets/*`：动画资源路径与帧收集逻辑。
- **交互层**：
  - `src/drag.rs`：长按拖拽、捏捏区域判断、窗口跟随与拖拽动画触发。
  - `src/interaction/*`：输入探针、右键菜单、输入区域裁剪、头/身体触摸区域判定。
- **状态展示层**：`src/ui/*`
  - `src/ui/stats/panel.rs`：状态面板渲染（`StatsPanel`，Popover）。
  - `src/ui/food/drug_panel.rs`：投喂分类面板渲染（`FeedPanel`，浮动 Window + 网格）。
- **状态计算层**：`src/stats/*`
  - `model.rs`：纯数据结构与纯计算函数（`PetStats`、`PetMode`、`InteractType`、模式判断与等级公式）。
  - `food.rs`：通用物品模型（`ItemDef`/`ItemEffects`/`ItemKind`）。
  - `service.rs`：带副作用的状态服务（衰减、投喂、互动、升级、配置上限应用）。
  - `mod.rs`：状态模块统一导出。
- **设置与窗口层**：
  - `src/settings/*`：设置模型、设置面板、持久化存储。
    - `model.rs`：`AppSettings` 包含 `remember_position`（bool）和 `scale_factor`（f64，范围 0.5~2.0）。
    - `panel.rs`：设置 UI，包含水平滑块（50%~200%）、百分比标签、恢复默认按钮。
    - `storage.rs`：通过 `settings/user_settings.toml` 持久化缩放因子。
  - `src/window/position.rs`：窗口位置读写与应用。

### 2.2 资源分层

- `assets/body/...` 按动作和状态组织（如 `StartUP`、`Default`、`Raise`、`Pinch`、`Touch_Head`、`Shutdown` 等）。
- 当前运行时不再使用 `IDEL` 与 `State` 动画逻辑；默认待机来自 `Default/*`。
- `config.toml` 提供资源根目录和子路径映射，支持替换资源目录布局。

## 3. 启动流程（高层）

1. `main()` 初始化 `Application` 并进入 `build_ui()`。
2. `build_ui()` 创建透明 Layer-Shell 窗口并设置默认锚点（右下）。
3. 加载面板配置并创建 `PetStatsService`（包含初始值和面板上限）。
4. 调用 `animation::load_carousel_images()`：
   - 构建各类 player；
   - 设置首帧；
  - 启动动画定时循环（`CAROUSEL_INTERVAL_MS`，当前 130ms）。
5. 启动独立 stats 定时器（周期取 `stats_service.logic_interval_secs()`，默认 5 秒）。
6. 绑定交互：输入探针、长按拖拽、头/身体点击区域、右键菜单（投喂分类子菜单 + 系统子菜单）。
7. 创建并连接 `StatsPanel`，并启动配置文件热更新监听。

## 4. 动画系统设计

### 4.1 请求机制（弱耦合）

交互层不会直接操作 player，而是写入 `animation/requests.rs` 中的原子请求位：

- Drag：start / loop / end
- Pinch：start / loop / end
- Touch：head / body
- Shutdown：request

动画调度器每个 tick 使用 `consume_requests()` 一次性消费请求，避免 UI 回调和动画状态机强耦合。

状态衰减不再绑定动画帧率：`main.rs` 里使用独立定时器按 `logic_interval_secs` 调用 `stats_service.on_tick(logic_interval_secs)`。

### 4.2 调度优先级

在 `coordinator` 中，事件分发优先级为：

1. `drag_raise`（长按拖动最高优先级，可抢占并打断其他动画）
2. `shutdown`
3. `pinch`
4. `touch`

帧推进优先级为：

1. `shutdown`
2. `drag_raise`
3. `pinch`
4. `touch`
5. `startup`
6. `side_hide_right_main`
7. `default_idle`

说明：`startup` 仅初始化时活跃，播完后自动回落到 `default_idle`。当前无 `IDEL`/`State` 分支。

### 4.6 右边界 SideHide（SideHide_Right_Main）

- 触发条件：仅按 `side_hide_right_trigger_pixel_x`（图片像素坐标映射后）判断是否靠近屏幕右边界。
- 进入后会将窗口贴靠右边界，并以 `side_hide_right_anchor_pixel_x / side_hide_right_anchor_pixel_y` 作为显示对齐参考点。
- 播放链路：`A -> B_1/B_2/B_3/B_4 轮播 -> ... -> C(被打断时)`。
- B 段按目录排序轮播（如 B_1 -> B_2 -> B_3 -> B_4 -> B_1 ...）；若某模式缺少部分编号，则在现有目录中循环。
- B 段不再使用时长限制；每个分段播完即切换到下一个分段。
- 在 `side_hide_right_main` 活跃期间，不触发 `IDEL/State`；收到 `Pinch/TouchHead/TouchBody` 请求时会先播 `C` 段收尾。

### 4.7 悬浮 SideHide（SideHide_Right_Rise）

- `side_hide_right_rise` 仅在 `side_hide_right_main` 已处于活跃状态时，鼠标悬浮才会触发。
- 普通待机状态下的 hover 不会直接触发 `rise`。
- 鼠标离开人物区域时发送结束请求，`rise` 播放 `C` 段收尾。
- 若触发时 `side_hide_right_main` 正在播放，则 `rise` 结束后会继续播放 `main` 未完成的进度，不会从头重播。
- 资源根目录通过 `config.toml` 中的 `side_hide_right_rise_root` 配置（默认 `SideHide_Right_Rise`）。

### 4.9 SideHide 越界中断规则（新增）

- 当宠物被拖拽离开右边界触发范围时，`side_hide_right_main` 与 `side_hide_right_rise` 会立即停止。
- 该停止策略为“强制 stop”，不会等待 `C` 段收尾播放完成。

### 4.8 SideHide_Right_Main 的 B_1 重播规则

- 针对 `assets/body/SideHide_Right_Main/Happy/B_1`：在该分段被轮播到时，额外有 `50%` 概率重播一次。
- 重播判定仅对上述路径生效，其他 SideHide 分段仍按常规轮播推进。

### 4.3 Default 待机播放规则

- `default_idle` 只播放 `Default` 资源。
- 按顺序拼接并循环（例如 Happy 为 `1 -> 2 -> 3 -> 1 -> ...`），不再随机单目录循环。
- 状态切换（Happy/Nomal/PoorCondition/Ill）时调用 `reload` 切换到对应目录序列。

### 4.4 拖拽 Raise 动画规则（新增）

- 拖拽开始后先播放 `Raised_Dynamic` 循环。
- 在持续拖拽过程中按播放 tick 计时；达到阈值后触发一次 `Raised_Static` 周期：
  1. 播放 `A_*` 一轮；
  2. 播放 `B_*` 3~7 轮（当前实现 B 段额外帧停留，视觉更慢）；
  3. 周期完成后回到 `Raised_Dynamic` 并重新计时。
- 若在 `Raised_Static` 播放期间放下鼠标，仍按结束流程播放 `C_*` 结束段。
- `A/B/C` 资源按模式命名匹配（如 `A_Happy`、`B_Nomal`），并带回退策略（优先当前 mode，后回退 Nomal/Happy）。
- 长按拖动触发后会立即打断当前动画（含 `shutdown`、`startup`、`pinch`、`touch`），确保窗口拖拽响应优先。
- `C_*` 结束段不再是不可中断；后续 `drag/shutdown/pinch/touch` 请求均可抢占并中断其播放。
- 当 `drag` 请求在 `C_*` 期间到达时，会立即中断 `C_*` 并切回 `Raised_Dynamic` 循环。

### 4.5 模式驱动资源切换

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

- 在头部矩形区域先执行 `on_interact(InteractType::TouchHead)`；仅当返回 `true` 时触发 `request_touch_head_animation()`。
- 在身体矩形区域先执行 `on_interact(InteractType::TouchBody)`；仅当返回 `true` 时触发 `request_touch_body_animation()`。
- 不同 `PetMode` 使用不同矩形参数，匹配不同体态资源。

## 6. 状态面板与配置热更新

### 6.1 状态面板

- `StatsPanel` 通过 `Popover` 显示体力、饱腹、口渴、心情、健康、好感、经验、等级与模式。
- `StatsPanel` 只读 `stats_service.get_stats()` 与 `stats_service.cal_mode()`，不持有业务状态逻辑。
- 右键菜单中的“面板”按钮用于显示/隐藏。

### 6.1.1 投喂分类面板（新增）

- 投喂入口在右键菜单 `投喂` 子菜单，包含：`主食/饮品/零食/礼物/药物/功能`。
- 每个分类由 `FeedPanel` 以浮动 `Window` 展示，UI 风格与“系统->设置”一致。
- 面板主体为 `Grid`（井字排版）+ `ScrolledWindow`，图片来源于 `assets/image/food/<category>`。
- 点击物品后读取对应 LPS 配置并调用 `PetStatsService::on_use_item` 生效，同时刷新状态面板并持久化存档。
- LPS 读取映射：
  - 主食：`food.lps` + `timelimit.lps`（`type=Meal`）
  - 饮品：`food.lps` + `moredrink.lps` + `timelimit.lps`（`type=Drink`）
  - 零食：`food.lps` + `timelimit.lps`（`type=Snack`）
  - 礼物：`gift.lps` + `timelimit.lps`（`type=Gift`）
  - 药物：`drug.lps`（`type=Drug`）
  - 功能：`food.lps` + `timelimit.lps`（`type=Functional`）

### 6.2 配置热更新

`config.toml` 变更后流程：

1. `start_panel_config_watcher()` 监听配置文件变更；
2. 通知主线程 channel；
3. 主线程定时轮询并执行：
   - `load_panel_debug_config()`
   - `stats_service.apply_panel_config(...)`
   - `stats_panel.refresh()`

这保证了面板参数可在运行时无重启生效；其中 `basic_stat_max / experience_max / level_max` 直接影响面板显示上限，`default_*` 影响当前状态初始值/替换值。

同时，动画参数也支持热更新：

1. 配置监听触发后，主线程调用 `request_animation_config_reload()`；
2. `coordinator` 在 tick 中消费该请求并重建 players；
3. 新配置即时生效（保留当前运行主流程，不重复播放 startup）。

### 6.3 当前数值规则（stats）

- `PetStats` 当前包含 `likability` 与 `likability_max`；`likability_max` 由等级决定：
  - `likability_max_for_level(level) = 90.0 + level * 10.0`
- 升级时同步刷新上限：`feeling_max`、`strength_max`、`likability_max`。
- `on_interact`（互动）流程：
  1. 先判定体力门槛：`strength >= 10`。
  2. 若体力不足：不触发动画，不改数值（返回 `false`）。
  3. 若体力足够但心情已满：仅触发动画，不改数值（返回 `true`）。
  4. 若体力足够且心情未满：执行统一效果（`strength -= 2`、`feeling += 1`、`exp += level`），并返回 `true`。
  5. 成功互动（返回 `true`）会重置“距离上次互动秒数”。
- 投喂统一入口为 `on_use_item(&ItemDef)`（`on_feed` 仅保留别名转发）：
  1. 先加 `likability`（带溢出转健康）；
  2. 再加 `feeling`（通过 `apply_feeling_gain` 联动好感）；
  3. 应用 `StrengthFood/StrengthDrink/Strength/Health/Exp` 等字段并统一 clamp/升级。
- `on_tick`（时间推进）新增自动消耗：
  1. 维持基础衰减（饱食/口渴/体力自然下降）。
  2. 当 `strength_food >= basic_stat_max * 50%`：额外消耗饱食并等量恢复体力（“消耗食物换体力”）。
  3. 当 `strength_food <= basic_stat_max * 25%`：按 `rand(0..1) * TimePass` 随机扣减健康。
- 心情随时间下降（基于上次互动时间）：
  - `freedrop = DECAY_BALANCE_FEELING * TimePass * idle_multiplier`，其中 `idle_multiplier` 随“距离上次互动秒数”增长并封顶；
  - 互动越久未发生，心情下降越快；成功互动后该计时重置。
- 心情与好感联动：
  - Feeling 正增益会等额转为好感增益（不再乘系数）；
  - Tick 中先计算 `raw_feeling`（不先截断）；当 `raw_feeling < 0` 时按 `raw_feeling / 2.0` 扣减好感，再统一 clamp；
  - 互动导致体力降到 0 不会立即触发额外心情衰减，仍由下一次 `on_tick` 统一处理；
  - 好感超上限溢出会转换为健康恢复（上限 100）。

## 6.4 缩放功能（Scale）

宠物窗口和图片大小可动态缩放，实现视觉上的人物大小变化。缩放范围为 50%~200%，默认 100%。

**缩放工作流程**：

1. **初始化**：
   - `main.rs` 启动时从 `SettingsStore::scale_factor()` 读取持久化因子（默认 1.0）。
   - 计算 `initial_pixel_size = (DEFAULT_PIXEL_SIZE × scale_factor).round().max(32)` 传给 `load_carousel_images()`。
   - GTK Image 控件通过 `image.set_pixel_size(pixel_size)` 确定渲染尺寸；窗口大小由内容自动决定。

2. **滑块预览**：
   - 设置面板中滑块变化时，实时调用 `on_scale_preview` 回调。
   - 回调直接调用 `image.set_pixel_size(新尺寸)`，用户看到实时预览。
   - 预览不会立即保存到 TOML；仅在按"保存"时才持久化。

3. **恢复默认**：
   - 点击"恢复默认"按钮将滑块重置为 100%。
   - 不自动保存；用户仍需点"保存"确认。

4. **保存与持久化**：
   - 点"保存"时调用 `SettingsStore::update_scale_factor(factor)`。
   - 因子写入 `settings/user_settings.toml` 的 `scale_factor` 字段。
   - 同时触发 `stats_service` 更新（如有设置变更），并通知动画配置重新加载。

5. **取消/退出/关闭**：
   - 取消、退出或关闭面板时，滑块回滚到上次保存的值（预览被撤销）。
   - 实时调用预览回调，确保最后看到的是已保存态。

**坐标系与交互适配**：

- 所有触摸区域、拖拽焦点、捏捏判定区域都基于"源图像像素"定义，使用 `widget_size / pixbuf_size` 比率自动映射到当前渲染尺寸。
- 缩放不改变这些逻辑坐标，只改变渲染 pixel_size，因此交互命中精度不受影响。
- 例：`focus_pixel_x = 581` (源坐标) → widget 显示宽 512px 时，映射到 widget 坐标约 342px。

**输入区域刷新**：

- 每帧动画 tick 时，`setup_image_input_region()` 会基于当前渲染尺寸重算 alpha 通道输入区域。
- 缩放后 alpha 区域自动缩放，确保可点击区域始终与可见宠物范围一致。

## 8. 关键设计取舍

- **事件与播放解耦**：请求位 + tick 消费，降低回调复杂度。
- **计算与渲染解耦**：`stats/model.rs` 保持纯计算，`ui/stats/panel.rs` 仅渲染，便于测试与维护。
- **单线程 UI 安全**：GTK 相关操作在主线程，避免线程访问 UI 风险。
- **资源路径可配置**：动画目录通过 `config.toml` 管理，便于换皮/重组资源。
- **输入区域跟帧同步**：提高交互命中精度，但会增加每帧 region 更新成本。

## 9. 典型扩展点

- 新增动作：
  1. 在 `player/` 增加对应 player；
  2. 在 `requests.rs` 增加请求位；
  3. 在 `coordinator.rs` 增加分发与优先级规则；
  4. 在 `assets/` 增加资源收集逻辑与配置项。
- 新增玩法逻辑：在 `PetStatsService` 中扩展 `on_tick/on_feed/on_interact` 并让动画/面板消费。
- 配置化交互区域：可将头/身体/捏捏矩形迁移到 `config.toml`，减少硬编码常量。


## 10. 动画链路与配置（2026-03）

- IDEL/State 动画已重新接入，支持三段式（A_Start/B_Loop/C_End/Single）与 StateONE <-> StateTWO 循环。
- 资源路径可配置：`idel_root`、`state_root`、`switch_up_root`、`switch_down_root`，详见 config.toml。
- State 动画帧率较低（默认 200ms），循环持续时长加倍（更慢更持久）。
- DefaultIdlePlayer 内部为状态机，支持 Default/Idel/StateONE/StateTWO/Switch 过渡。
- 逻辑定时器每 15 秒触发，按概率分支进入 Idle/State/Move/Sleep/RandomInteraction。
- 模式切换时自动播放 Switch_Up/Down 过渡动画，逐级递归，结束回 Default。

## 11. 当前架构一句话总结

该项目采用“GTK 主线程 + 双定时器（动画/状态） + 原子请求队列 + 多播放器状态机 + 可配置资源路径”的结构，在保持交互响应的同时，实现了可热更、可扩展的桌宠动画系统。
