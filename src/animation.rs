use glib::timeout_add_local;
use gtk4::{ApplicationWindow, Image};
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use crate::config::CAROUSEL_INTERVAL_MS;
use crate::input_region::setup_image_input_region;

pub fn load_carousel_images(
    window: &ApplicationWindow,
    current_pixbuf: Rc<RefCell<Option<gdk_pixbuf::Pixbuf>>>,
) -> Result<Image, String> {
    let asset_dir = PathBuf::from("/home/jialuo/Code/jialuoPet/assets/body/Default/Happy/1");

    if !asset_dir.is_dir() {
        return Err(format!("目录不存在：{}", asset_dir.display()));
    }

    let mut image_files: Vec<PathBuf> = fs::read_dir(&asset_dir)
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

    if image_files.is_empty() {
        return Err(format!("目录中没有找到 PNG 文件：{}", asset_dir.display()));
    }

    image_files.sort();

    let image = Image::new();
    image.set_pixel_size(256);

    let state = Rc::new(RefCell::new((0usize, image_files.clone())));
    let state_clone = state.clone();
    let image_clone = image.clone();
    let window_clone = window.clone();

    if let Ok(pixbuf) = gdk_pixbuf::Pixbuf::from_file(&image_files[0]) {
        image_clone.set_from_pixbuf(Some(&pixbuf));
        *current_pixbuf.borrow_mut() = Some(pixbuf.clone());
        setup_image_input_region(&window_clone, &image_clone, &pixbuf);
    }

    timeout_add_local(Duration::from_millis(CAROUSEL_INTERVAL_MS), move || {
        let next_path = {
            let mut state_mut = state_clone.borrow_mut();
            let current_index = state_mut.0;
            let image_paths = &state_mut.1;
            let next_index = (current_index + 1) % image_paths.len();
            let next_path = image_paths[next_index].clone();
            state_mut.0 = next_index;
            next_path
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