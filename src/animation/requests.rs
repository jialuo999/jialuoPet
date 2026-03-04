// ===== 依赖导入 =====
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

// ===== 各类动画请求阶段常量 =====
pub(crate) const DRAG_ANIM_IDLE: u8 = 0;
pub(crate) const DRAG_ANIM_START_REQUESTED: u8 = 1;
pub(crate) const DRAG_ANIM_LOOP_REQUESTED: u8 = 2;
pub(crate) const DRAG_ANIM_END_REQUESTED: u8 = 3;

pub(crate) const PINCH_ANIM_IDLE: u8 = 0;
pub(crate) const PINCH_ANIM_START_REQUESTED: u8 = 1;
pub(crate) const PINCH_ANIM_LOOP_REQUESTED: u8 = 2;
pub(crate) const PINCH_ANIM_END_REQUESTED: u8 = 3;

pub(crate) const SHUTDOWN_ANIM_IDLE: u8 = 0;
pub(crate) const SHUTDOWN_ANIM_REQUESTED: u8 = 1;

pub(crate) const TOUCH_ANIM_IDLE: u8 = 0;
pub(crate) const TOUCH_ANIM_HEAD_REQUESTED: u8 = 1;
pub(crate) const TOUCH_ANIM_BODY_REQUESTED: u8 = 2;

pub(crate) const HOVER_ANIM_IDLE: u8 = 0;
pub(crate) const HOVER_ANIM_START_REQUESTED: u8 = 1;
pub(crate) const HOVER_ANIM_END_REQUESTED: u8 = 2;

// ===== 全局请求状态（原子变量） =====
static DRAG_RAISE_ANIMATION_PHASE: AtomicU8 = AtomicU8::new(DRAG_ANIM_IDLE);
static PINCH_ANIMATION_PHASE: AtomicU8 = AtomicU8::new(PINCH_ANIM_IDLE);
static SHUTDOWN_ANIMATION_PHASE: AtomicU8 = AtomicU8::new(SHUTDOWN_ANIM_IDLE);
static TOUCH_ANIMATION_PHASE: AtomicU8 = AtomicU8::new(TOUCH_ANIM_IDLE);
static HOVER_ANIMATION_PHASE: AtomicU8 = AtomicU8::new(HOVER_ANIM_IDLE);
static SHUTDOWN_ANIMATION_FINISHED: AtomicBool = AtomicBool::new(false);
static ANIMATION_CONFIG_RELOAD_REQUESTED: AtomicBool = AtomicBool::new(false);

// ===== 单帧消费的请求快照 =====
pub(crate) struct AnimationRequests {
    pub(crate) drag: u8,
    pub(crate) pinch: u8,
    pub(crate) shutdown: u8,
    pub(crate) touch: u8,
    pub(crate) hover: u8,
}

// ===== 请求写入接口 =====
pub fn request_drag_raise_animation_start() {
    DRAG_RAISE_ANIMATION_PHASE.store(DRAG_ANIM_START_REQUESTED, Ordering::Relaxed);
}

pub fn request_drag_raise_animation_loop() {
    DRAG_RAISE_ANIMATION_PHASE.store(DRAG_ANIM_LOOP_REQUESTED, Ordering::Relaxed);
}

pub fn request_drag_raise_animation_end() {
    DRAG_RAISE_ANIMATION_PHASE.store(DRAG_ANIM_END_REQUESTED, Ordering::Relaxed);
}

pub fn request_pinch_animation_start() {
    PINCH_ANIMATION_PHASE.store(PINCH_ANIM_START_REQUESTED, Ordering::Relaxed);
}

pub fn request_pinch_animation_end() {
    PINCH_ANIMATION_PHASE.store(PINCH_ANIM_END_REQUESTED, Ordering::Relaxed);
}

pub fn request_shutdown_animation() {
    SHUTDOWN_ANIMATION_FINISHED.store(false, Ordering::Relaxed);
    SHUTDOWN_ANIMATION_PHASE.store(SHUTDOWN_ANIM_REQUESTED, Ordering::Relaxed);
}

pub fn request_touch_head_animation() {
    TOUCH_ANIMATION_PHASE.store(TOUCH_ANIM_HEAD_REQUESTED, Ordering::Relaxed);
}

pub fn request_touch_body_animation() {
    TOUCH_ANIMATION_PHASE.store(TOUCH_ANIM_BODY_REQUESTED, Ordering::Relaxed);
}

pub fn request_hover_animation_start() {
    HOVER_ANIMATION_PHASE.store(HOVER_ANIM_START_REQUESTED, Ordering::Relaxed);
}

pub fn request_hover_animation_end() {
    HOVER_ANIMATION_PHASE.store(HOVER_ANIM_END_REQUESTED, Ordering::Relaxed);
}

pub fn request_animation_config_reload() {
    ANIMATION_CONFIG_RELOAD_REQUESTED.store(true, Ordering::Relaxed);
}

// ===== 状态读取/消费接口 =====
pub fn is_shutdown_animation_finished() -> bool {
    SHUTDOWN_ANIMATION_FINISHED.load(Ordering::Relaxed)
}

pub(crate) fn set_shutdown_animation_finished(value: bool) {
    SHUTDOWN_ANIMATION_FINISHED.store(value, Ordering::Relaxed);
}

pub(crate) fn consume_requests() -> AnimationRequests {
    AnimationRequests {
        drag: DRAG_RAISE_ANIMATION_PHASE.swap(DRAG_ANIM_IDLE, Ordering::Relaxed),
        pinch: PINCH_ANIMATION_PHASE.swap(PINCH_ANIM_IDLE, Ordering::Relaxed),
        shutdown: SHUTDOWN_ANIMATION_PHASE.swap(SHUTDOWN_ANIM_IDLE, Ordering::Relaxed),
        touch: TOUCH_ANIMATION_PHASE.swap(TOUCH_ANIM_IDLE, Ordering::Relaxed),
        hover: HOVER_ANIMATION_PHASE.swap(HOVER_ANIM_IDLE, Ordering::Relaxed),
    }
}

pub(crate) fn consume_animation_config_reload_request() -> bool {
    ANIMATION_CONFIG_RELOAD_REQUESTED.swap(false, Ordering::Relaxed)
}
