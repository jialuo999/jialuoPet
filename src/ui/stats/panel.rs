// ===== 依赖导入 =====
use gtk4::prelude::*;
use gtk4::{Align, Box, Image, Label, Orientation, Popover, ProgressBar};

use crate::stats::PetStatsService;

// ===== 状态面板组件 =====
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
    level_value: Label,
    money_value: Label,
}

impl StatsPanel {
    pub fn new(image: &Image, stats_service: PetStatsService) -> Self {
        // ===== 面板基础容器 =====
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

        let title = Label::new(Some("角色属性"));
        title.set_halign(Align::Start);
        root.append(&title);

        let (stamina_row, stamina_bar, stamina_value) = build_stat_row("体力");
        let (satiety_row, satiety_bar, satiety_value) = build_stat_row("饱腹度");
        let (thirst_row, thirst_bar, thirst_value) = build_stat_row("口渴度");
        let (mood_row, mood_bar, mood_value) = build_stat_row("心情");
        let (health_row, health_bar, health_value) = build_stat_row("健康");
        let (affinity_row, affinity_bar, affinity_value) = build_stat_row("好感度");
        let (experience_row, experience_bar, experience_value) = build_stat_row("经验值");
        let (level_row, level_value) = build_value_row("等级");
        let (money_row, money_value) = build_value_row("金钱");

        root.append(&stamina_row);
        root.append(&satiety_row);
        root.append(&thirst_row);
        root.append(&mood_row);
        root.append(&health_row);
        root.append(&affinity_row);
        root.append(&experience_row);
        root.append(&level_row);
        root.append(&money_row);

        popover.set_child(Some(&root));

        // ===== 组件装配 =====
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
            level_value,
            money_value,
        };

        panel.refresh();
        panel
    }

    // ===== 面板显示控制 =====
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

    // ===== 数据刷新 =====
    pub fn refresh(&self) {
        let stats = self.stats_service.get_stats();
        let experience_need = stats.level_up_exp_needed();

        set_bar_value(
            &self.stamina_bar,
            &self.stamina_value,
            stats.strength,
            stats.strength_max,
        );
        set_bar_value(
            &self.satiety_bar,
            &self.satiety_value,
            stats.strength_food,
            stats.strength_max,
        );
        set_bar_value(
            &self.thirst_bar,
            &self.thirst_value,
            stats.strength_drink,
            stats.strength_max,
        );
        set_bar_value(&self.mood_bar, &self.mood_value, stats.feeling, stats.feeling_max);
        set_bar_value(
            &self.health_bar,
            &self.health_value,
            stats.health,
            100.0,
        );
        set_bar_value(
            &self.affinity_bar,
            &self.affinity_value,
            stats.likability,
            stats.likability_max,
        );
        set_bar_value(
            &self.experience_bar,
            &self.experience_value,
            stats.exp,
            experience_need,
        );
        self.level_value.set_text(&format!("Lev:{}", stats.level));
        self.money_value.set_text(&format!("{}", stats.money));
    }
}

// ===== UI 辅助函数 =====
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

fn build_value_row(name: &str) -> (Box, Label) {
    let row = Box::new(Orientation::Horizontal, 6);

    let label = Label::new(Some(name));
    label.set_halign(Align::Start);
    label.set_width_chars(6);

    let value = Label::new(None);
    value.set_halign(Align::End);
    value.set_hexpand(true);

    row.append(&label);
    row.append(&value);

    (row, value)
}

fn set_bar_value(bar: &ProgressBar, value_label: &Label, value: f64, max: f64) {
    let max_value = max.max(f64::EPSILON);
    let current = value.clamp(0.0, max_value);
    bar.set_fraction(current / max_value);
    value_label.set_text(&format!("{:.0}/{:.0}", current, max_value));
}

