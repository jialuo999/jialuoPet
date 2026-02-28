use gtk4::prelude::*;
use gtk4::{Align, Application, ApplicationWindow, Box, Label, Orientation, ProgressBar};
use std::cell::RefCell;
use std::rc::Rc;

const BASIC_STAT_MAX: u32 = 100;
const EXPERIENCE_MAX: u32 = 100;
const LEVEL_MAX: u32 = 100;

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
            stamina: 80,
            satiety: 70,
            thirst: 65,
            mood: 75,
            health: 90,
            affinity: 50,
            experience: 10,
            level: 1,
        }
    }
}

#[derive(Clone)]
pub struct PetStatsService {
    inner: Rc<RefCell<PetStats>>,
}

impl PetStatsService {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(PetStats::default())),
        }
    }

    pub fn snapshot(&self) -> PetStats {
        self.inner.borrow().clone()
    }

    #[allow(dead_code)]
    pub fn set_stamina(&self, value: u32) {
        self.inner.borrow_mut().stamina = value.min(BASIC_STAT_MAX);
    }

    #[allow(dead_code)]
    pub fn set_satiety(&self, value: u32) {
        self.inner.borrow_mut().satiety = value.min(BASIC_STAT_MAX);
    }

    #[allow(dead_code)]
    pub fn set_thirst(&self, value: u32) {
        self.inner.borrow_mut().thirst = value.min(BASIC_STAT_MAX);
    }

    #[allow(dead_code)]
    pub fn set_mood(&self, value: u32) {
        self.inner.borrow_mut().mood = value.min(BASIC_STAT_MAX);
    }

    #[allow(dead_code)]
    pub fn set_health(&self, value: u32) {
        self.inner.borrow_mut().health = value.min(BASIC_STAT_MAX);
    }

    #[allow(dead_code)]
    pub fn set_affinity(&self, value: u32) {
        self.inner.borrow_mut().affinity = value.min(BASIC_STAT_MAX);
    }

    #[allow(dead_code)]
    pub fn set_experience(&self, value: u32) {
        self.inner.borrow_mut().experience = value.min(EXPERIENCE_MAX);
    }

    #[allow(dead_code)]
    pub fn set_level(&self, value: u32) {
        self.inner.borrow_mut().level = value.min(LEVEL_MAX);
    }

    #[allow(dead_code)]
    pub fn gain_experience(&self, amount: u32) {
        let mut stats = self.inner.borrow_mut();
        stats.experience = (stats.experience + amount).min(EXPERIENCE_MAX);
    }

    #[allow(dead_code)]
    pub fn on_feed(&self, amount: u32) {
        let mut stats = self.inner.borrow_mut();
        stats.satiety = (stats.satiety + amount).min(BASIC_STAT_MAX);
    }

    #[allow(dead_code)]
    pub fn on_drink(&self, amount: u32) {
        let mut stats = self.inner.borrow_mut();
        stats.thirst = (stats.thirst + amount).min(BASIC_STAT_MAX);
    }

    #[allow(dead_code)]
    pub fn on_interact(&self, amount: u32) {
        let mut stats = self.inner.borrow_mut();
        stats.mood = (stats.mood + amount).min(BASIC_STAT_MAX);
        stats.affinity = (stats.affinity + amount / 2).min(BASIC_STAT_MAX);
    }

    #[allow(dead_code)]
    pub fn on_tick(&self) {}
}

pub struct StatsPanel {
    window: ApplicationWindow,
    stats_service: PetStatsService,
    stamina_bar: ProgressBar,
    satiety_bar: ProgressBar,
    thirst_bar: ProgressBar,
    mood_bar: ProgressBar,
    health_bar: ProgressBar,
    affinity_bar: ProgressBar,
    experience_bar: ProgressBar,
    level_bar: ProgressBar,
}

impl StatsPanel {
    pub fn new(app: &Application, stats_service: PetStatsService) -> Self {
        let window = ApplicationWindow::new(app);
        window.set_title(Some("角色面板"));
        window.set_default_size(360, 360);
        window.set_hide_on_close(true);

        let root = Box::new(Orientation::Vertical, 10);
        root.set_margin_top(12);
        root.set_margin_bottom(12);
        root.set_margin_start(12);
        root.set_margin_end(12);

        let title = Label::new(Some("角色健康系统"));
        title.set_halign(Align::Start);
        root.append(&title);

        let (stamina_row, stamina_bar) = build_stat_row("体力");
        let (satiety_row, satiety_bar) = build_stat_row("饱腹度");
        let (thirst_row, thirst_bar) = build_stat_row("口渴度");
        let (mood_row, mood_bar) = build_stat_row("心情");
        let (health_row, health_bar) = build_stat_row("健康");
        let (affinity_row, affinity_bar) = build_stat_row("好感度");
        let (experience_row, experience_bar) = build_stat_row("经验值");
        let (level_row, level_bar) = build_stat_row("等级");

        root.append(&stamina_row);
        root.append(&satiety_row);
        root.append(&thirst_row);
        root.append(&mood_row);
        root.append(&health_row);
        root.append(&affinity_row);
        root.append(&experience_row);
        root.append(&level_row);

        window.set_child(Some(&root));

        let panel = Self {
            window,
            stats_service,
            stamina_bar,
            satiety_bar,
            thirst_bar,
            mood_bar,
            health_bar,
            affinity_bar,
            experience_bar,
            level_bar,
        };

        panel.refresh();
        panel
    }

    pub fn present(&self) {
        self.refresh();
        self.window.present();
    }

    pub fn refresh(&self) {
        let stats = self.stats_service.snapshot();
        set_bar_value(&self.stamina_bar, stats.stamina, BASIC_STAT_MAX);
        set_bar_value(&self.satiety_bar, stats.satiety, BASIC_STAT_MAX);
        set_bar_value(&self.thirst_bar, stats.thirst, BASIC_STAT_MAX);
        set_bar_value(&self.mood_bar, stats.mood, BASIC_STAT_MAX);
        set_bar_value(&self.health_bar, stats.health, BASIC_STAT_MAX);
        set_bar_value(&self.affinity_bar, stats.affinity, BASIC_STAT_MAX);
        set_bar_value(&self.experience_bar, stats.experience, EXPERIENCE_MAX);
        set_bar_value(&self.level_bar, stats.level, LEVEL_MAX);
    }
}

fn build_stat_row(name: &str) -> (Box, ProgressBar) {
    let row = Box::new(Orientation::Vertical, 4);

    let label = Label::new(Some(name));
    label.set_halign(Align::Start);

    let bar = ProgressBar::new();
    bar.set_show_text(true);

    row.append(&label);
    row.append(&bar);

    (row, bar)
}

fn set_bar_value(bar: &ProgressBar, value: u32, max: u32) {
    let max_value = max.max(1) as f64;
    let current = value.min(max) as f64;
    bar.set_fraction(current / max_value);
    bar.set_text(Some(&format!("{}/{}", value.min(max), max)));
}
