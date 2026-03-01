use std::path::{Path, PathBuf};

use crate::stats_panel::PetMode;

use super::AnimationPlayer;
use crate::animation::assets::{
    collect_dir_paths, collect_png_files, dir_name_matches_mode, pseudo_random_index,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum IdlePhase {
    A,
    B,
    C,
    Single,
}

pub(crate) struct IdlePlayer {
    idle_root: PathBuf,
    current_mode: PetMode,
    active: bool,
    phase: IdlePhase,
    files: Vec<PathBuf>,
    index: usize,
    loop_count: u8,
}

impl IdlePlayer {
    pub(crate) fn new(idle_root: PathBuf, mode: PetMode) -> Self {
        Self {
            idle_root,
            current_mode: mode,
            active: false,
            phase: IdlePhase::Single,
            files: Vec::new(),
            index: 0,
            loop_count: 0,
        }
    }

    pub(crate) fn start(&mut self, startup: &mut super::StartupPlayer) {
        startup.stop();
        
        // 1. 寻找所有可能的动画变体目录（如 aside, meowlook, yawning）
        let variants = collect_dir_paths(&self.idle_root);
        if variants.is_empty() {
            return;
        }

        // 2. 随机选择一个变体
        let variant_path = &variants[pseudo_random_index(variants.len())];
        
        // 3. 在变体目录下寻找匹配当前状态的目录
        let mode_dirs = collect_dir_paths(variant_path);
        let mut selected_mode_dir = mode_dirs.iter().find(|p| {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or_default();
            dir_name_matches_mode(name, self.current_mode)
        });

        // 降级逻辑：若对应状态帧缺失，降级到 Nomal
        if selected_mode_dir.is_none() && self.current_mode != PetMode::Nomal {
            selected_mode_dir = mode_dirs.iter().find(|p| {
                let name = p.file_name().and_then(|s| s.to_str()).unwrap_or_default();
                dir_name_matches_mode(name, PetMode::Nomal)
            });
        }

        let Some(mode_dir) = selected_mode_dir else {
            return;
        };

        // 4. 检查是 ABC 三段式还是 Single 单段
        let sub_dirs = collect_dir_paths(mode_dir);
        let has_abc = sub_dirs.iter().any(|p| {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or_default().to_lowercase();
            name.starts_with('a') || name.contains("start")
        });

        if has_abc {
            // ABC 三段式
            self.start_abc(mode_dir);
        } else {
            // Single 单段
            let files = collect_png_files(mode_dir).unwrap_or_default();
            if !files.is_empty() {
                self.files = files;
                self.index = 0;
                self.phase = IdlePhase::Single;
                self.active = true;
            }
        }
    }

    fn start_abc(&mut self, mode_dir: &Path) {
        let sub_dirs = collect_dir_paths(mode_dir);
        
        // 查找 A 段
        let a_dir = sub_dirs.iter().find(|p| {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or_default().to_lowercase();
            name.starts_with('a') || name.contains("start")
        });

        if let Some(dir) = a_dir {
            let files = collect_png_files(dir).unwrap_or_default();
            if !files.is_empty() {
                self.files = files;
                self.index = 0;
                self.phase = IdlePhase::A;
                self.active = true;
                return;
            }
        }

        // 如果没有 A 段，直接尝试 B 段
        self.transition_to_b(mode_dir);
    }

    fn transition_to_b(&mut self, mode_dir: &Path) {
        let sub_dirs = collect_dir_paths(mode_dir);
        let b_dir = sub_dirs.iter().find(|p| {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or_default().to_lowercase();
            name.starts_with('b') || name.contains("loop")
        });

        if let Some(dir) = b_dir {
            let files = collect_png_files(dir).unwrap_or_default();
            if !files.is_empty() {
                self.files = files;
                self.index = 0;
                self.phase = IdlePhase::B;
                self.active = true;
                // 随机循环次数 2-4 次
                self.loop_count = 2 + pseudo_random_index(3) as u8;
                return;
            }
        }

        // 如果没有 B 段，尝试 C 段
        self.transition_to_c(mode_dir);
    }

    fn transition_to_c(&mut self, mode_dir: &Path) {
        let sub_dirs = collect_dir_paths(mode_dir);
        let c_dir = sub_dirs.iter().find(|p| {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or_default().to_lowercase();
            name.starts_with('c') || name.contains("end")
        });

        if let Some(dir) = c_dir {
            let files = collect_png_files(dir).unwrap_or_default();
            if !files.is_empty() {
                self.files = files;
                self.index = 0;
                self.phase = IdlePhase::C;
                self.active = true;
                return;
            }
        }

        // 如果连 C 段都没有，结束
        self.active = false;
    }

    // 获取当前动画所属的 mode_dir
    fn get_current_mode_dir(&self) -> Option<PathBuf> {
        if self.files.is_empty() {
            return None;
        }
        // self.files[0] 是 .../IDEL/variant/Mode/A/frame.png 或 .../IDEL/variant/Mode/frame.png
        let parent = self.files[0].parent()?;
        match self.phase {
            IdlePhase::A | IdlePhase::B | IdlePhase::C => Some(parent.parent()?.to_path_buf()),
            IdlePhase::Single => Some(parent.to_path_buf()),
        }
    }
}

impl AnimationPlayer for IdlePlayer {
    fn is_active(&self) -> bool {
        self.active
    }

    fn next_frame(&mut self) -> Option<PathBuf> {
        if !self.active || self.files.is_empty() {
            return None;
        }

        let frame = self.files[self.index].clone();
        self.index += 1;

        if self.index >= self.files.len() {
            match self.phase {
                IdlePhase::A => {
                    if let Some(mode_dir) = self.get_current_mode_dir() {
                        self.transition_to_b(&mode_dir);
                    } else {
                        self.active = false;
                    }
                }
                IdlePhase::B => {
                    if self.loop_count > 1 {
                        self.loop_count -= 1;
                        self.index = 0;
                    } else if let Some(mode_dir) = self.get_current_mode_dir() {
                        self.transition_to_c(&mode_dir);
                    } else {
                        self.active = false;
                    }
                }
                IdlePhase::C | IdlePhase::Single => {
                    self.active = false;
                }
            }
        }

        Some(frame)
    }

    fn interrupt(&mut self, skip_to_end: bool) {
        if !self.active {
            return;
        }

        if skip_to_end && self.phase == IdlePhase::B {
            // 如果在 B 段被中断，且要求播放结束段，则切换到 C 段
            if let Some(mode_dir) = self.get_current_mode_dir() {
                self.transition_to_c(&mode_dir);
                if !self.active {
                    // 如果没有 C 段，则直接停止
                    self.active = false;
                }
            } else {
                self.active = false;
            }
        } else if self.phase == IdlePhase::C {
            // C 段不可被中断（强制播放）
            // 这里不做处理，让 next_frame 继续播放
        } else {
            self.active = false;
        }
    }

    fn reload(&mut self, mode: PetMode) {
        self.current_mode = mode;
    }
}
