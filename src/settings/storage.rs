use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;

use super::model::{AppSettings, WindowPosition};

const SETTINGS_STATE_FILE: &str = "settings/user_settings.toml";

pub struct SettingsStore {
    file_path: PathBuf,
    settings: RefCell<AppSettings>,
}

impl SettingsStore {
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

    pub fn update_remember_position(&self, enabled: bool) -> anyhow::Result<()> {
        {
            let mut settings = self.settings.borrow_mut();
            settings.remember_position = enabled;
        }
        self.persist()
    }

    pub fn update_position(&self, left: i32, top: i32) -> anyhow::Result<()> {
        {
            let mut settings = self.settings.borrow_mut();
            settings.window_position = Some(WindowPosition { left, top });
        }
        self.persist()
    }

    fn persist(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let serialized = toml::to_string_pretty(&*self.settings.borrow())?;
        fs::write(&self.file_path, serialized)?;
        Ok(())
    }
}
