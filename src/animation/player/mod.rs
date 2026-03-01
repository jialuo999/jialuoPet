mod default_idle;
mod drag_raise;
mod pinch;
mod shutdown;
mod startup;
mod touch;

use std::path::PathBuf;

use crate::stats::PetMode;

pub(crate) use default_idle::DefaultIdlePlayer;
pub(crate) use drag_raise::DragRaisePlayer;
pub(crate) use pinch::PinchPlayer;
pub(crate) use shutdown::ShutdownPlayer;
pub(crate) use startup::StartupPlayer;
pub(crate) use touch::TouchPlayer;

pub(crate) trait AnimationPlayer {
    fn is_active(&self) -> bool;
    fn next_frame(&mut self) -> Option<PathBuf>;
    fn interrupt(&mut self, skip_to_end: bool);
    fn stop(&mut self) {
        self.interrupt(true);
    }
    fn reload(&mut self, mode: PetMode);
}
