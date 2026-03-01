# 动画播放结构与流程（当前实现）

## 主循环（每 130ms）

`coordinator` 定时执行四步：

1. `consume_requests()`：消费 drag/pinch/touch/shutdown 的原子请求
2. `maybe_update_mode()`：根据状态值决定是否重载各 player 资源
3. `dispatch_requests()`：按优先级切换 player 状态
4. `advance_frame()`：按取帧优先级选择下一帧路径并刷新图像

## 两层优先级

- `dispatch` 优先级：`shutdown > drag_raise > pinch > touch`
- `advance_frame` 取帧优先级：`shutdown -> drag_raise -> pinch -> touch -> startup -> default_idle`

说明：`startup` 不参与外部请求调度，只在初始化时激活一次，播完后自动让位给 `default_idle`。

## 播放结束回落规则

- 非 `default_idle` 的 player 在播完最后一帧后，`next_frame()` 返回 `None` 且 `is_active` 变为 `false`
- `coordinator::advance_frame()` 捕获该状态后会调用 `default_idle.enter()`，立即回到待机链路

## Drag Start 资源策略

- 当前资源集中未提供 drag start 段，因此 `collect_drag_raise_start_files()` 返回空
- `DragRaisePlayer::start()` 在 start 为空时会自动降级进入 loop
- 后续补齐 start 资源后，只需在资源收集函数中实现 start 段加载即可
