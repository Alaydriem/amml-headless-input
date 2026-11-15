use std::path::PathBuf;

use super::layer::ScreenshotLayer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenshotShot {
    pub png: Vec<u8>,
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub source_width: u32,
    pub source_height: u32,
    pub layer: ScreenshotLayer,
}
