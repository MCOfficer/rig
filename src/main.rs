use std::env::current_dir;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::Result;
use argh::FromArgs;
use fltk::enums::Event;
use image::RgbImage;
use log::{error, info};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use walkdir::WalkDir;

fn setup_logger() {
    use simplelog::*;
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )])
    .unwrap();
}

#[derive(FromArgs, Debug)]
/// A recursive image gallery.
struct Opts {
    /// files or directories to load
    #[argh(positional)]
    paths: Vec<PathBuf>,
}

fn main() {
    setup_logger();
    let opts: Opts = argh::from_env();
    let mut paths = opts.paths;
    if paths.is_empty() {
        paths.push(current_dir().unwrap());
    }
    let images = find_images(paths);
    gui(images);
}

fn gui(images: Vec<PathBuf>) {
    use fltk::prelude::*;
    use fltk::*;

    let images = Arc::new(images);
    let index = AtomicUsize::new(images.len() - 1); // We start at the end of the list, so we can move 1 forward to out actual startign point (0)

    let app = app::App::default().with_scheme(app::Scheme::Plastic);
    let mut wind = window::Window::default()
        .with_pos(100, 100)
        .with_size(1280, 720)
        .with_label("Hello from rust");
    wind.make_resizable(true);

    let mut image_frame = frame::Frame::default();
    load_image(&images, &index, &mut image_frame, false);
    image_frame.handle(|f, event| {
        if let Event::Resize = event {
            let image = f.image().unwrap();
            let (x, y, width, height) = compute_dimensions(
                f.window().unwrap().w() as f32,
                f.window().unwrap().h() as f32,
                image.data_w() as f32,
                image.data_h() as f32,
            );

            f.set_size(width, height);
            f.set_pos(x, y);
            f.set_image::<fltk::image::Image>(None);
            f.set_image_scaled(Some(image.to_rgb().unwrap()));
            true
        } else {
            false
        }
    });

    wind.end();
    wind.show();

    wind.handle(move |wind, event| match event {
        enums::Event::Push => {
            match app::event_button() {
                1 => load_image(&images, &index, &mut image_frame, true),
                3 => load_image(&images, &index, &mut image_frame, false),
                _ => {}
            };
            true
        }
        enums::Event::KeyDown => {
            if app::event_key() == enums::Key::Right {
                load_image(&images, &index, &mut image_frame, false);
            } else if app::event_key() == enums::Key::Left {
                load_image(&images, &index, &mut image_frame, true);
            } else if app::event_key() == enums::Key::from_char('f') {
                wind.fullscreen(!wind.fullscreen_active());
            } else if app::event_key() == enums::Key::from_char('q') {
                app::quit();
            };

            true
        }
        _ => false,
    });
    app.run().unwrap();
}

fn move_index(images: &[PathBuf], atomic: &AtomicUsize, prev: bool) {
    let index = atomic.load(Ordering::SeqCst);
    let new = if prev && index > 0 {
        index - 1
    } else if prev && index == 0 {
        images.len() - 1
    } else if index == images.len() - 1 {
        0
    } else {
        index + 1
    };

    atomic.store(new, Ordering::SeqCst);
}

fn load_image<F>(images: &[PathBuf], atomic: &AtomicUsize, frame: &mut F, prev: bool)
where
    F: fltk::prelude::WidgetExt,
{
    let rgb = loop {
        move_index(images, atomic, prev);
        let index = atomic.load(Ordering::SeqCst);
        let path = images.get(index).unwrap();
        match try_load_raw(path) {
            Ok(result) => {
                break result;
            }
            Err(e) => {
                error!("Failed to load {}: {}", path.to_string_lossy(), e)
            }
        }
    };

    let image = fltk::image::RgbImage::new(
        rgb.as_raw(),
        rgb.width() as i32,
        rgb.height() as i32,
        fltk::enums::ColorDepth::Rgb8,
    )
    .unwrap();
    frame.set_image::<fltk::image::Image>(None);
    frame.set_image_scaled(Some(image));

    // Force a resize so our resize callback runs, then redraw
    let mut wind = frame.window().unwrap();
    wind.resize(wind.x(), wind.y(), wind.w(), wind.h());
    wind.redraw();
}

fn try_load_raw(path: &Path) -> Result<RgbImage> {
    let dynamic = image::open(path)?;
    Ok(dynamic.into_rgb8())
}

fn compute_dimensions(
    container_width: f32,
    container_height: f32,
    rgb_width: f32,
    rgb_height: f32,
) -> (i32, i32, i32, i32) {
    let container_ratio = container_width / container_height;
    let rgb_ratio = rgb_width / rgb_height;

    if rgb_ratio > container_ratio {
        let new_width = container_width;
        let scale = new_width / rgb_width;
        let new_height = rgb_height * scale;

        let new_x = 0;
        let new_y = (container_height - new_height) / 2_f32;

        (
            new_x as i32,
            new_y as i32,
            new_width as i32,
            new_height as i32,
        )
    } else {
        let new_height = container_height;
        let scale = new_height / rgb_height;
        let new_width = rgb_width * scale;

        let new_x = (container_width - new_width) / 2_f32;
        let new_y = 0;

        (
            new_x as i32,
            new_y as i32,
            new_width as i32,
            new_height as i32,
        )
    }
}

fn find_images(input_paths: Vec<PathBuf>) -> Vec<PathBuf> {
    info!("Searching for files in {} paths", input_paths.len());
    let mut files = vec![];
    for path in input_paths {
        if path.is_file() {
            files.push(path);
        } else {
            for res in WalkDir::new(&path) {
                match res {
                    Ok(entry) => {
                        if entry.file_type().is_file() {
                            files.push(entry.into_path())
                        }
                    }
                    Err(e) => error!("Failed to read directory {}: {}", path.to_string_lossy(), e),
                };
            }
        }
    }

    info!("Guessing MIME types of {} files", files.len());
    let mut unknown = vec![];

    // We need the map so we can push actual objects into `unknown`, `filter` only gives us references
    #[allow(clippy::unnecessary_filter_map)]
    let mut images: Vec<PathBuf> = files
        .drain(..)
        .filter_map(|path| {
            let guess = new_mime_guess::from_path(&path);
            if guess.first_or_octet_stream().type_() == "image" {
                Some(path)
            } else {
                unknown.push(path);
                None
            }
        })
        .collect();

    info!("Inferring MIME types of {} files", unknown.len());
    for path in unknown {
        if is_image(&path).unwrap_or(false) {
            images.push(path)
        }
    }

    info!("Found {} images", images.len());
    images
}

fn is_image(path: &Path) -> Result<bool> {
    let file = std::fs::File::open(path)?;
    let limit = file
        .metadata()
        .map(|m| std::cmp::min(m.len(), 8192) as usize + 1)
        .unwrap_or(0);
    let mut buffer = Vec::with_capacity(limit);
    file.take(8192).read_to_end(&mut buffer)?;
    Ok(infer::is_image(&buffer))
}
