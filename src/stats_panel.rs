use gtk4::prelude::*;
use gtk4::{Align, Box, Image, Label, Orientation, Popover, ProgressBar};
use std::cell::RefCell;
use std::rc::Rc;

use crate::config::PanelDebugConfig;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PetMode {
    Happy,
    Nomal,
    PoorCondition,
    Ill,
}

impl PetMode {
    pub fn label(self) -> &'static str {
        match self {
            PetMode::Happy => "Happy",
            PetMode::Nomal => "Nomal",
            PetMode::PoorCondition => "PoorCondition",
            PetMode::Ill => "Ill",
        }
    }
}

#[derive(Clone)]
pub struct PetStats {
    pub stamina: u32,
    pub satiety: u32,
    pub thirst: u32,
    pub mood: u32,
    pub health: u32,
    pub affinity: u32,
    pub experience: u32,
    pub level: u32,
}

impl Default for PetStats {
    fn default() -> Self {
        Self {
            stamina: 0,
            satiety: 0,
            thirst: 0,
            mood: 0,
            health: 0,
            affinity: 0,
            experience: 0,
            level: 0,
        }
    }
}

fn stats_from_panel_config(panel_config: &PanelDebugConfig) -> PetStats {
    PetStats {
        stamina: panel_config.default_stamina,
        satiety: panel_config.default_satiety,
        thirst: panel_config.default_thirst,
        mood: panel_config.default_mood,
        health: panel_config.default_health,
        affinity: panel_config.default_affinity,
        experience: panel_config.default_experience,
        level: panel_config.default_level,
    }
}

#[derive(Clone)]
pub struct PetStatsService {
    inner: Rc<RefCell<PetStats>>,
    panel_config: Rc<RefCell<PanelDebugConfig>>,
}

impl PetStatsService {
    pub fn new(panel_config: Rc<RefCell<PanelDebugConfig>>) -> Self {
        let initial_stats = {
            let config = panel_config.borrow().clone();
            stats_from_panel_config(&config)
        };

        Self {
            inner: Rc::new(RefCell::new(initial_stats)),
            panel_config,
        }
    }

    pub fn apply_panel_config(&self, panel_config: PanelDebugConfig) {
        *self.panel_config.borrow_mut() = panel_config.clone();
        *self.inner.borrow_mut() = stats_from_panel_config(&panel_config);
    }

    pub fn basic_stat_max(&self) -> u32 {
        self.panel_config.borrow().basic_stat_max
    }

    pub fn experience_max(&self) -> u32 {
        self.panel_config.borrow().experience_max
    }

    pub fn level_max(&self) -> u32 {
        self.panel_config.borrow().level_max
    }

    pub fn snapshot(&self) -> PetStats {
        self.inner.borrow().clone()
    }

    pub fn cal_mode(&self) -> PetMode {
        let stats = self.inner.borrow();
        let feeling_max = self.basic_stat_max().max(1);
        let feeling = stats.mood.min(feeling_max);
        let health = stats.health.min(feeling_max);
        let likability = stats.affinity.min(feeling_max);

        let feeling_percent = feeling * 100 / feeling_max;
        let mut realhel: i32 = 60;
        if feeling_percent >= 80 {
            realhel -= 12;
        }
        if likability >= 80 {
            realhel -= 12;
        } else if likability >= 40 {
            realhel -= 6;
        }
        realhel = realhel.max(0);

        let health_i32 = health as i32;
        if health_i32 <= realhel {
            return if health_i32 <= realhel / 2 {
                PetMode::Ill
            } else {
                PetMode::PoorCondition
            };
        }

        let realfel = 0.90
            - if likability >= 80 {
                0.20
            } else if likability >= 40 {
                0.10
            } else {
                0.0
            };
        let felps = feeling as f64 / feeling_max as f64;

        if felps >= realfel {
            PetMode::Happy
        } else if felps <= realfel / 2.0 {
            PetMode::PoorCondition
        } else {
            PetMode::Nomal
        }
    }

    #[allow(dead_code)]
    pub fn set_stamina(&self, value: u32) {
        self.inner.borrow_mut().stamina = value.min(self.basic_stat_max());
    }

    #[allow(dead_code)]
    pub fn set_satiety(&self, value: u32) {
        self.inner.borrow_mut().satiety = value.min(self.basic_stat_max());
    }

    #[allow(dead_code)]
    pub fn set_thirst(&self, value: u32) {
        self.inner.borrow_mut().thirst = value.min(self.basic_stat_max());
    }

    #[allow(dead_code)]
    pub fn set_mood(&self, value: u32) {
        self.inner.borrow_mut().mood = value.min(self.basic_stat_max());
    }

    #[allow(dead_code)]
    pub fn set_health(&self, value: u32) {
        self.inner.borrow_mut().health = value.min(self.basic_stat_max());
    }

    #[allow(dead_code)]
    pub fn set_affinity(&self, value: u32) {
        self.inner.borrow_mut().affinity = value.min(self.basic_stat_max());
    }

    #[allow(dead_code)]
    pub fn set_experience(&self, value: u32) {
        self.inner.borrow_mut().experience = value.min(self.experience_max());
    }

    #[allow(dead_code)]
    pub fn set_level(&self, value: u32) {
        self.inner.borrow_mut().level = value.min(self.level_max());
    }

    #[allow(dead_code)]
    pub fn gain_experience(&self, amount: u32) {
        let mut stats = self.inner.borrow_mut();
        stats.experience = (stats.experience + amount).min(self.experience_max());
    }

    #[allow(dead_code)]
    pub fn on_feed(&self, amount: u32) {
        let mut stats = self.inner.borrow_mut();
        stats.satiety = (stats.satiety + amount).min(self.basic_stat_max());
    }

    #[allow(dead_code)]
    pub fn on_drink(&self, amount: u32) {
        let mut stats = self.inner.borrow_mut();
        stats.thirst = (stats.thirst + amount).min(self.basic_stat_max());
    }

    #[allow(dead_code)]
    pub fn on_interact(&self, amount: u32) {
        let basic_stat_max = self.basic_stat_max();
        let mut stats = self.inner.borrow_mut();
        stats.mood = (stats.mood + amount).min(basic_stat_max);
        stats.affinity = (stats.affinity + amount / 2).min(basic_stat_max);
    }

    #[allow(dead_code)]
    pub fn on_tick(&self) {}
}

pub struct StatsPanel {
    popover: Popover,
    stats_service: PetStatsService,
    stamina_bar: ProgressBar,
    stamina_value: Label,
    satiety_bar: ProgressBar,
    satiety_value: Label,
    thirst_bar: ProgressBar,
    thirst_value: Label,
    mood_bar: ProgressBar,
    mood_value: Label,
    health_bar: ProgressBar,
    health_value: Label,
    affinity_bar: ProgressBar,
    affinity_value: Label,
    experience_bar: ProgressBar,
    experience_value: Label,
    level_bar: ProgressBar,
    level_value: Label,
    mode_value: Label,
}

impl StatsPanel {
    pub fn new(image: &Image, stats_service: PetStatsService) -> Self {
        let popover = Popover::new();
        popover.set_has_arrow(true);
        popover.set_autohide(false);
        popover.set_parent(image);

        let root = Box::new(Orientation::Vertical, 6);
        root.set_margin_top(8);
        root.set_margin_bottom(8);
        root.set_margin_start(8);
        root.set_margin_end(8);
        root.set_size_request(260, -1);

        let title = Label::new(Some("角色健康系统"));
        title.set_halign(Align::Start);
        root.append(&title);

        let mode_row = Box::new(Orientation::Horizontal, 6);
        let mode_title = Label::new(Some("状态"));
        mode_title.set_halign(Align::Start);
        mode_title.set_width_chars(6);
        let mode_value = Label::new(None);
        mode_value.set_halign(Align::End);
        mode_value.set_hexpand(true);
        mode_row.append(&mode_title);
        mode_row.append(&mode_value);
        root.append(&mode_row);

        let (stamina_row, stamina_bar, stamina_value) = build_stat_row("体力");
        let (satiety_row, satiety_bar, satiety_value) = build_stat_row("饱腹度");
        let (thirst_row, thirst_bar, thirst_value) = build_stat_row("口渴度");
        let (mood_row, mood_bar, mood_value) = build_stat_row("心情");
        let (health_row, health_bar, health_value) = build_stat_row("健康");
        let (affinity_row, affinity_bar, affinity_value) = build_stat_row("好感度");
        let (experience_row, experience_bar, experience_value) = build_stat_row("经验值");
        let (level_row, level_bar, level_value) = build_stat_row("等级");

        root.append(&stamina_row);
        root.append(&satiety_row);
        root.append(&thirst_row);
        root.append(&mood_row);
        root.append(&health_row);
        root.append(&affinity_row);
        root.append(&experience_row);
        root.append(&level_row);

        popover.set_child(Some(&root));

        let panel = Self {
            popover,
            stats_service,
            stamina_bar,
            stamina_value,
            satiety_bar,
            satiety_value,
            thirst_bar,
            thirst_value,
            mood_bar,
            mood_value,
            health_bar,
            health_value,
            affinity_bar,
            affinity_value,
            experience_bar,
            experience_value,
            level_bar,
            level_value,
            mode_value,
        };

        panel.refresh();
        panel
    }

    pub fn present_at(&self, x: i32, y: i32) {
        self.refresh();
        self.popover
            .set_pointing_to(Some(&gdk4::Rectangle::new(x, y, 1, 1)));
        self.popover.popup();
    }

    pub fn toggle_at(&self, x: i32, y: i32) {
        if self.popover.is_visible() {
            self.popover.popdown();
        } else {
            self.present_at(x, y);
        }
    }

    pub fn hide(&self) {
        self.popover.popdown();
    }

    pub fn refresh(&self) {
        let stats = self.stats_service.snapshot();
        let mode = self.stats_service.cal_mode();
        let basic_stat_max = self.stats_service.basic_stat_max();
        let experience_max = self.stats_service.experience_max();
        let level_max = self.stats_service.level_max();
        set_bar_value(
            &self.stamina_bar,
            &self.stamina_value,
            stats.stamina,
            basic_stat_max,
        );
        set_bar_value(
            &self.satiety_bar,
            &self.satiety_value,
            stats.satiety,
            basic_stat_max,
        );
        set_bar_value(
            &self.thirst_bar,
            &self.thirst_value,
            stats.thirst,
            basic_stat_max,
        );
        set_bar_value(&self.mood_bar, &self.mood_value, stats.mood, basic_stat_max);
        set_bar_value(
            &self.health_bar,
            &self.health_value,
            stats.health,
            basic_stat_max,
        );
        set_bar_value(
            &self.affinity_bar,
            &self.affinity_value,
            stats.affinity,
            basic_stat_max,
        );
        set_bar_value(
            &self.experience_bar,
            &self.experience_value,
            stats.experience,
            experience_max,
        );
        set_bar_value(&self.level_bar, &self.level_value, stats.level, level_max);
        self.mode_value.set_text(mode.label());
    }
}

fn build_stat_row(name: &str) -> (Box, ProgressBar, Label) {
    let row = Box::new(Orientation::Horizontal, 6);

    let label = Label::new(Some(name));
    label.set_halign(Align::Start);
    label.set_width_chars(6);

    let bar = ProgressBar::new();
    bar.set_show_text(false);
    bar.set_hexpand(true);
    bar.set_valign(Align::Center);

    let value = Label::new(None);
    value.set_width_chars(7);
    value.set_halign(Align::End);

    row.append(&label);
    row.append(&bar);
    row.append(&value);

    (row, bar, value)
}

fn set_bar_value(bar: &ProgressBar, value_label: &Label, value: u32, max: u32) {
    let max_value = max.max(1) as f64;
    let current = value.min(max) as f64;
    bar.set_fraction(current / max_value);
    value_label.set_text(&format!("{}/{}", value.min(max), max));
}
