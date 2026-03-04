// ===== player 子模块声明 =====
mod default_idle;
mod drag_raise;
mod pinch;
mod side_hide_right_main;
mod shutdown;
mod startup;
mod touch;

// ===== 公共依赖 =====
use std::path::PathBuf;

use crate::stats::PetMode;

// ===== 对内导出 =====
pub(crate) use default_idle::DefaultIdlePlayer;
pub(crate) use drag_raise::DragRaisePlayer;
pub(crate) use pinch::PinchPlayer;
pub(crate) use side_hide_right_main::SideHideRightMainPlayer;
pub(crate) use shutdown::ShutdownPlayer;
pub(crate) use startup::StartupPlayer;
pub(crate) use touch::TouchPlayer;

// ===== 播放器统一接口 =====
pub(crate) trait AnimationPlayer {
    fn is_active(&self) -> bool;
    fn next_frame(&mut self) -> Option<PathBuf>;
    fn interrupt(&mut self, skip_to_end: bool);
    fn stop(&mut self) {
        self.interrupt(true);
    }
    fn reload(&mut self, mode: PetMode);
}
