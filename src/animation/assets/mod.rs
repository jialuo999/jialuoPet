// ===== assets 子模块声明 =====
mod common;
mod default_idle;
mod drag_raise;
mod idel_state;
mod pinch;
mod side_hide;
mod shutdown;
mod startup;
mod touch;

// ===== 对外导出（保持旧 API 不变） =====
pub(crate) use common::body_asset_path;
pub(crate) use default_idle::{
    collect_default_happy_idle_variants, collect_default_mode_idle_variants,
    select_default_files_for_mode,
};
pub(crate) use drag_raise::{
    collect_drag_raise_end_variants, collect_drag_raise_loop_files,
    collect_drag_raise_start_files, collect_drag_raise_static_b_variants,
};
pub(crate) use idel_state::{
    collect_idel_action_names, load_idel_loop_variants, load_idel_segment,
    load_state_loop_variants, load_state_segment, load_switch_single,
    IdelStateSegment,
};
pub(crate) use pinch::{
    collect_pinch_end_files, collect_pinch_loop_variants, collect_pinch_start_files,
};
pub(crate) use side_hide::{
    collect_side_hide_end_files, collect_side_hide_loop_variants, collect_side_hide_start_files,
};
pub(crate) use shutdown::collect_shutdown_variants;
pub(crate) use startup::choose_startup_animation_files;
pub(crate) use touch::{build_touch_sequence, collect_touch_variants, TouchStageVariants};
pub(crate) use common::pseudo_random_index;
