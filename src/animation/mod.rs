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
    request_play_game_animation, request_play_remove_object_animation,
    request_play_rope_skipping_animation, request_play_stop_animation,
    request_pinch_animation_end, request_pinch_animation_start, request_shutdown_animation,
    request_study_book_animation, request_study_paint_animation,
    request_study_research_animation, request_study_stop_animation,
    request_work_clean_animation, request_work_copywriting_animation,
    request_work_stop_animation, request_work_streaming_animation,
    request_touch_body_animation, request_touch_head_animation,
};
