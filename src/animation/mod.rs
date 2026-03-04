// ===== animation 子模块声明 =====
mod assets;
mod coordinator;
mod player;
mod requests;

// ===== 对外导出 =====
pub use coordinator::load_carousel_images;
pub use requests::{
    is_shutdown_animation_finished, request_drag_raise_animation_end,
    request_drag_raise_animation_loop, request_drag_raise_animation_start,
    request_animation_config_reload,
    request_hover_animation_end, request_hover_animation_start,
    request_pinch_animation_end, request_pinch_animation_start, request_shutdown_animation,
    request_touch_body_animation, request_touch_head_animation,
};
