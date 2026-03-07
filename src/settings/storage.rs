// ===== 依赖导入 =====
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;

use super::model::{AppSettings, WindowPosition};

// ===== 持久化文件路径 =====
const SETTINGS_STATE_FILE: &str = "settings/user_settings.toml";

// ===== 设置存储服务 =====
pub struct SettingsStore {
    file_path: PathBuf,
    settings: RefCell<AppSettings>,
}

impl SettingsStore {
	// 从磁盘加载设置，不存在时回退默认值
    pub fn load() -> Self {
        let file_path = PathBuf::from(SETTINGS_STATE_FILE);
        let settings = fs::read_to_string(&file_path)
            .ok()
            .and_then(|content| toml::from_str::<AppSettings>(&content).ok())
            .unwrap_or_default();

        Self {
            file_path,
            settings: RefCell::new(settings),
        }
    }

	// 获取当前设置快照
    pub fn snapshot(&self) -> AppSettings {
        self.settings.borrow().clone()
    }

    pub fn remember_position_enabled(&self) -> bool {
        self.settings.borrow().remember_position
    }

    pub fn remembered_position_if_enabled(&self) -> Option<WindowPosition> {
        let settings = self.settings.borrow();
        if settings.remember_position {
            settings.window_position
        } else {
            None
        }
    }

    pub fn scale_factor(&self) -> f64 {
        self.settings.borrow().scale_factor
    }

    pub fn auto_close_panels_on_outside_click(&self) -> bool {
        self.settings.borrow().auto_close_panels_on_outside_click
    }

	// 更新缩放因子并持久化
    pub fn update_scale_factor(&self, factor: f64) -> anyhow::Result<()> {
        {
            let mut settings = self.settings.borrow_mut();
            settings.scale_factor = factor;
        }
        self.persist()
    }

	// 更新开关并持久化
    pub fn update_remember_position(&self, enabled: bool) -> anyhow::Result<()> {
        {
            let mut settings = self.settings.borrow_mut();
            settings.remember_position = enabled;
        }
        self.persist()
    }

    pub fn update_auto_close_panels_on_outside_click(&self, enabled: bool) -> anyhow::Result<()> {
        {
            let mut settings = self.settings.borrow_mut();
            settings.auto_close_panels_on_outside_click = enabled;
        }
        self.persist()
    }

	// 更新位置并持久化
    pub fn update_position(&self, left: i32, top: i32) -> anyhow::Result<()> {
        {
            let mut settings = self.settings.borrow_mut();
            settings.window_position = Some(WindowPosition { left, top });
        }
        self.persist()
    }

	// 统一写盘入口
    fn persist(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let serialized = toml::to_string_pretty(&*self.settings.borrow())?;
        fs::write(&self.file_path, serialized)?;
        Ok(())
    }
}
