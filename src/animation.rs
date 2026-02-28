use glib::timeout_add_local;
use gtk4::{ApplicationWindow, Image};
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::{CAROUSEL_INTERVAL_MS, STARTUP_EXCLUDED_DIRS};
use crate::input_region::setup_image_input_region;

static DRAG_RAISE_ANIMATION_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn set_drag_raise_animation_active(active: bool) {
    DRAG_RAISE_ANIMATION_ACTIVE.store(active, Ordering::Relaxed);
}

fn collect_png_files(asset_dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !asset_dir.is_dir() {
        return Err(format!("目录不存在：{}", asset_dir.display()));
    }

    let mut image_files: Vec<PathBuf> = fs::read_dir(asset_dir)
        .map_err(|e| format!("无法读取目录 {}: {}", asset_dir.display(), e))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("png") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    image_files.sort();
    Ok(image_files)
}

fn collect_png_files_recursive_filtered(
    asset_dir: &Path,
    excluded_dirs: &[&str],
) -> Result<Vec<PathBuf>, String> {
    if !asset_dir.is_dir() {
        return Err(format!("目录不存在：{}", asset_dir.display()));
    }

    fn visit_dir(
        current_dir: &Path,
        excluded_dirs: &[&str],
        output: &mut Vec<PathBuf>,
    ) -> Result<(), String> {
        let entries = fs::read_dir(current_dir)
            .map_err(|e| format!("无法读取目录 {}: {}", current_dir.display(), e))?;

        for entry in entries {
            let entry = match entry {
                Ok(value) => value,
                Err(_) => continue,
            };
            let path = entry.path();

            if path.is_dir() {
                let dir_name = match path.file_name().and_then(|s| s.to_str()) {
                    Some(value) => value,
                    None => continue,
                };
                if excluded_dirs
                    .iter()
                    .any(|excluded| dir_name.eq_ignore_ascii_case(excluded))
                {
                    continue;
                }
                visit_dir(&path, excluded_dirs, output)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("png") {
                output.push(path);
            }
        }

        Ok(())
    }

    let mut image_files = Vec::new();
    visit_dir(asset_dir, excluded_dirs, &mut image_files)?;
    image_files.sort();
    Ok(image_files)
}

fn choose_startup_animation_files(startup_root: &Path) -> Option<Vec<PathBuf>> {
    let startup_dirs: Vec<PathBuf> = fs::read_dir(startup_root)
        .ok()?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if !path.is_dir() {
                return None;
            }

            let dir_name = path.file_name()?.to_str()?;
            if STARTUP_EXCLUDED_DIRS
                .iter()
                .any(|excluded| dir_name.eq_ignore_ascii_case(excluded))
            {
                return None;
            }

            Some(path)
        })
        .collect();

    if startup_dirs.is_empty() {
        return None;
    }

    let mut available_variants: Vec<Vec<PathBuf>> = startup_dirs
        .iter()
        .filter_map(|dir| {
            let files = collect_png_files(dir).ok()?;
            if files.is_empty() {
                None
            } else {
                Some(files)
            }
        })
        .collect();

    if available_variants.is_empty() {
        return None;
    }

    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as usize;
    let selected_index = seed % available_variants.len();
    Some(available_variants.swap_remove(selected_index))
}

fn collect_drag_raise_happy_files(raise_dynamic_root: &Path) -> Vec<PathBuf> {
    let happy_dir = raise_dynamic_root.join("Happy");
    if happy_dir.is_dir() {
        return collect_png_files_recursive_filtered(&happy_dir, STARTUP_EXCLUDED_DIRS)
            .unwrap_or_default();
    }

    Vec::new()
}

struct CarouselState {
    startup_files: Vec<PathBuf>,
    startup_index: usize,
    default_files: Vec<PathBuf>,
    default_index: usize,
    drag_raise_files: Vec<PathBuf>,
    drag_raise_index: usize,
    playing_startup: bool,
}

pub fn load_carousel_images(
    window: &ApplicationWindow,
    current_pixbuf: Rc<RefCell<Option<gdk_pixbuf::Pixbuf>>>,
) -> Result<Image, String> {
    let default_dir = PathBuf::from("/home/jialuo/Code/jialuoPet/assets/body/Default/Happy/1");
    let default_files = collect_png_files(&default_dir)?;
    if default_files.is_empty() {
        return Err(format!("目录中没有找到 PNG 文件：{}", default_dir.display()));
    }

    let startup_root = PathBuf::from("/home/jialuo/Code/jialuoPet/assets/body/StartUP");
    let startup_files = choose_startup_animation_files(&startup_root).unwrap_or_default();
    let playing_startup = !startup_files.is_empty();
    let drag_raise_dir = PathBuf::from("/home/jialuo/Code/jialuoPet/assets/body/Raise/Raised_Dynamic");
    let drag_raise_files = collect_drag_raise_happy_files(&drag_raise_dir);

    let image = Image::new();
    image.set_pixel_size(256);

    let state = Rc::new(RefCell::new(CarouselState {
        startup_files,
        startup_index: 0,
        default_files,
        default_index: 0,
        drag_raise_files,
        drag_raise_index: 0,
        playing_startup,
    }));
    let state_clone = state.clone();
    let image_clone = image.clone();
    let window_clone = window.clone();

    let first_frame = {
        let state_ref = state.borrow();
        if state_ref.playing_startup {
            state_ref.startup_files[0].clone()
        } else {
            state_ref.default_files[0].clone()
        }
    };

    if let Ok(pixbuf) = gdk_pixbuf::Pixbuf::from_file(&first_frame) {
        image_clone.set_from_pixbuf(Some(&pixbuf));
        *current_pixbuf.borrow_mut() = Some(pixbuf.clone());
        setup_image_input_region(&window_clone, &image_clone, &pixbuf);
    }

    timeout_add_local(Duration::from_millis(CAROUSEL_INTERVAL_MS), move || {
        let next_path = {
            let mut state_mut = state_clone.borrow_mut();
            let dragging = DRAG_RAISE_ANIMATION_ACTIVE.load(Ordering::Relaxed);
            if dragging && !state_mut.drag_raise_files.is_empty() {
                let next_index = (state_mut.drag_raise_index + 1) % state_mut.drag_raise_files.len();
                state_mut.drag_raise_index = next_index;
                state_mut.drag_raise_files[next_index].clone()
            } else if state_mut.playing_startup {
                let next_startup_index = state_mut.startup_index + 1;
                if next_startup_index < state_mut.startup_files.len() {
                    state_mut.startup_index = next_startup_index;
                    state_mut.startup_files[next_startup_index].clone()
                } else {
                    state_mut.playing_startup = false;
                    state_mut.default_index = 0;
                    state_mut.default_files[0].clone()
                }
            } else {
                let next_index = (state_mut.default_index + 1) % state_mut.default_files.len();
                state_mut.default_index = next_index;
                state_mut.default_files[next_index].clone()
            }
        };

        if let Ok(pixbuf) = gdk_pixbuf::Pixbuf::from_file(&next_path) {
            image_clone.set_from_pixbuf(Some(&pixbuf));
            *current_pixbuf.borrow_mut() = Some(pixbuf.clone());
            setup_image_input_region(&window_clone, &image_clone, &pixbuf);
        }

        glib::ControlFlow::Continue
    });

    Ok(image)
}