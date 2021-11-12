use image::RgbImage;
use log::{error, info};
use parking_lot::RwLock;
use std::path::PathBuf;

impl From<PathBuf> for ImageFile {
    fn from(path: PathBuf) -> Self {
        Self {
            path,
            rgb: RwLock::new(None),
        }
    }
}

#[derive(Debug)]
enum LoadedImage {
    Failed,
    Loaded(RgbImage),
}

impl From<Option<RgbImage>> for LoadedImage {
    fn from(o: Option<RgbImage>) -> Self {
        match o {
            None => Self::Failed,
            Some(rgb) => Self::Loaded(rgb),
        }
    }
}

pub struct ImageFile {
    pub path: PathBuf,
    rgb: RwLock<Option<LoadedImage>>,
}

impl ImageFile {
    pub fn load(&self) -> Option<RgbImage> {
        self.preload();
        match self.rgb.read().as_ref() {
            None => {
                error!("RefCell was not initialized despite preload");
                None
            }
            Some(res) => match res {
                LoadedImage::Failed => None,
                LoadedImage::Loaded(rgb) => Some(rgb.clone()),
            },
        }
    }

    pub fn preload(&self) {
        if self.rgb.read().is_none() {
            let mut guard = self.rgb.write();
            info!("Preloading {}", self.path.to_string_lossy());
            let loaded = image::open(&self.path)
                .map(|i| i.into_rgb8())
                .map_err(|e| error!("Failed to preload {}: {}", self.path.to_string_lossy(), e))
                .ok()
                .into();
            *guard = Some(loaded);
        }
    }
}
