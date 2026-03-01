use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

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

static DRAG_RAISE_ANIMATION_PHASE: AtomicU8 = AtomicU8::new(DRAG_ANIM_IDLE);
static PINCH_ANIMATION_PHASE: AtomicU8 = AtomicU8::new(PINCH_ANIM_IDLE);
static SHUTDOWN_ANIMATION_PHASE: AtomicU8 = AtomicU8::new(SHUTDOWN_ANIM_IDLE);
static TOUCH_ANIMATION_PHASE: AtomicU8 = AtomicU8::new(TOUCH_ANIM_IDLE);
static SHUTDOWN_ANIMATION_FINISHED: AtomicBool = AtomicBool::new(false);

pub(crate) struct AnimationRequests {
    pub(crate) drag: u8,
    pub(crate) pinch: u8,
    pub(crate) shutdown: u8,
    pub(crate) touch: u8,
}

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
    }
}
