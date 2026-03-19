// ===== 依赖导入 =====
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};

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

pub(crate) const STUDY_ANIM_IDLE: u8 = 0;
pub(crate) const STUDY_ANIM_BOOK_REQUESTED: u8 = 1;
pub(crate) const STUDY_ANIM_PAINT_REQUESTED: u8 = 2;
pub(crate) const STUDY_ANIM_RESEARCH_REQUESTED: u8 = 3;
pub(crate) const STUDY_ANIM_STOP_REQUESTED: u8 = 4;

pub(crate) const WORK_ANIM_IDLE: u8 = 0;
pub(crate) const WORK_ANIM_CLEAN_REQUESTED: u8 = 1;
pub(crate) const WORK_ANIM_COPYWRITING_REQUESTED: u8 = 2;
pub(crate) const WORK_ANIM_STREAMING_REQUESTED: u8 = 3;
pub(crate) const WORK_ANIM_STOP_REQUESTED: u8 = 4;

pub(crate) const PLAY_ANIM_IDLE: u8 = 0;
pub(crate) const PLAY_ANIM_GAME_REQUESTED: u8 = 1;
pub(crate) const PLAY_ANIM_REMOVE_OBJECT_REQUESTED: u8 = 2;
pub(crate) const PLAY_ANIM_ROPE_SKIPPING_REQUESTED: u8 = 3;
pub(crate) const PLAY_ANIM_STOP_REQUESTED: u8 = 4;

// ===== 全局请求状态（原子变量） =====
static DRAG_RAISE_ANIMATION_PHASE: AtomicU8 = AtomicU8::new(DRAG_ANIM_IDLE);
static PINCH_ANIMATION_PHASE: AtomicU8 = AtomicU8::new(PINCH_ANIM_IDLE);
static SHUTDOWN_ANIMATION_PHASE: AtomicU8 = AtomicU8::new(SHUTDOWN_ANIM_IDLE);
static TOUCH_ANIMATION_PHASE: AtomicU8 = AtomicU8::new(TOUCH_ANIM_IDLE);
static HOVER_ANIMATION_PHASE: AtomicU8 = AtomicU8::new(HOVER_ANIM_IDLE);
static STUDY_ANIMATION_PHASE: AtomicU8 = AtomicU8::new(STUDY_ANIM_IDLE);
static STUDY_ANIMATION_DURATION_SECS: AtomicU32 = AtomicU32::new(1800);
static WORK_ANIMATION_PHASE: AtomicU8 = AtomicU8::new(WORK_ANIM_IDLE);
static WORK_ANIMATION_DURATION_SECS: AtomicU32 = AtomicU32::new(1800);
static PLAY_ANIMATION_PHASE: AtomicU8 = AtomicU8::new(PLAY_ANIM_IDLE);
static PLAY_ANIMATION_DURATION_SECS: AtomicU32 = AtomicU32::new(1800);
static SHUTDOWN_ANIMATION_FINISHED: AtomicBool = AtomicBool::new(false);
static ANIMATION_CONFIG_RELOAD_REQUESTED: AtomicBool = AtomicBool::new(false);

// ===== 单帧消费的请求快照 =====
pub(crate) struct AnimationRequests {
    pub(crate) drag: u8,
    pub(crate) pinch: u8,
    pub(crate) shutdown: u8,
    pub(crate) touch: u8,
    pub(crate) hover: u8,
    pub(crate) study: u8,
    pub(crate) study_duration_secs: u32,
    pub(crate) work: u8,
    pub(crate) work_duration_secs: u32,
    pub(crate) play: u8,
    pub(crate) play_duration_secs: u32,
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

pub fn request_study_book_animation(duration_secs: u32) {
    STUDY_ANIMATION_DURATION_SECS.store(duration_secs.max(1), Ordering::Relaxed);
    STUDY_ANIMATION_PHASE.store(STUDY_ANIM_BOOK_REQUESTED, Ordering::Relaxed);
}

pub fn request_study_paint_animation(duration_secs: u32) {
    STUDY_ANIMATION_DURATION_SECS.store(duration_secs.max(1), Ordering::Relaxed);
    STUDY_ANIMATION_PHASE.store(STUDY_ANIM_PAINT_REQUESTED, Ordering::Relaxed);
}

pub fn request_study_research_animation(duration_secs: u32) {
    STUDY_ANIMATION_DURATION_SECS.store(duration_secs.max(1), Ordering::Relaxed);
    STUDY_ANIMATION_PHASE.store(STUDY_ANIM_RESEARCH_REQUESTED, Ordering::Relaxed);
}

pub fn request_study_stop_animation() {
    STUDY_ANIMATION_PHASE.store(STUDY_ANIM_STOP_REQUESTED, Ordering::Relaxed);
}

pub fn request_work_clean_animation(duration_secs: u32) {
    WORK_ANIMATION_DURATION_SECS.store(duration_secs.max(1), Ordering::Relaxed);
    WORK_ANIMATION_PHASE.store(WORK_ANIM_CLEAN_REQUESTED, Ordering::Relaxed);
}

pub fn request_work_copywriting_animation(duration_secs: u32) {
    WORK_ANIMATION_DURATION_SECS.store(duration_secs.max(1), Ordering::Relaxed);
    WORK_ANIMATION_PHASE.store(WORK_ANIM_COPYWRITING_REQUESTED, Ordering::Relaxed);
}

pub fn request_work_streaming_animation(duration_secs: u32) {
    WORK_ANIMATION_DURATION_SECS.store(duration_secs.max(1), Ordering::Relaxed);
    WORK_ANIMATION_PHASE.store(WORK_ANIM_STREAMING_REQUESTED, Ordering::Relaxed);
}

pub fn request_work_stop_animation() {
    WORK_ANIMATION_PHASE.store(WORK_ANIM_STOP_REQUESTED, Ordering::Relaxed);
}

pub fn request_play_game_animation(duration_secs: u32) {
    PLAY_ANIMATION_DURATION_SECS.store(duration_secs.max(1), Ordering::Relaxed);
    PLAY_ANIMATION_PHASE.store(PLAY_ANIM_GAME_REQUESTED, Ordering::Relaxed);
}

pub fn request_play_remove_object_animation(duration_secs: u32) {
    PLAY_ANIMATION_DURATION_SECS.store(duration_secs.max(1), Ordering::Relaxed);
    PLAY_ANIMATION_PHASE.store(PLAY_ANIM_REMOVE_OBJECT_REQUESTED, Ordering::Relaxed);
}

pub fn request_play_rope_skipping_animation(duration_secs: u32) {
    PLAY_ANIMATION_DURATION_SECS.store(duration_secs.max(1), Ordering::Relaxed);
    PLAY_ANIMATION_PHASE.store(PLAY_ANIM_ROPE_SKIPPING_REQUESTED, Ordering::Relaxed);
}

pub fn request_play_stop_animation() {
    PLAY_ANIMATION_PHASE.store(PLAY_ANIM_STOP_REQUESTED, Ordering::Relaxed);
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
        study: STUDY_ANIMATION_PHASE.swap(STUDY_ANIM_IDLE, Ordering::Relaxed),
        study_duration_secs: STUDY_ANIMATION_DURATION_SECS.load(Ordering::Relaxed),
        work: WORK_ANIMATION_PHASE.swap(WORK_ANIM_IDLE, Ordering::Relaxed),
        work_duration_secs: WORK_ANIMATION_DURATION_SECS.load(Ordering::Relaxed),
        play: PLAY_ANIMATION_PHASE.swap(PLAY_ANIM_IDLE, Ordering::Relaxed),
        play_duration_secs: PLAY_ANIMATION_DURATION_SECS.load(Ordering::Relaxed),
    }
}

pub(crate) fn consume_animation_config_reload_request() -> bool {
    ANIMATION_CONFIG_RELOAD_REQUESTED.swap(false, Ordering::Relaxed)
}
