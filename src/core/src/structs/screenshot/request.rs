use super::layer::ScreenshotLayer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenshotRequest {
    pub layer: ScreenshotLayer,
    pub max_width: u32,
}

impl ScreenshotRequest {
    pub const DEFAULT_MAX_WIDTH: u32 = 1280;
    pub const MIN_WIDTH: u32 = 64;
    pub const MAX_WIDTH: u32 = 3840;

    pub fn parse(layer: Option<&str>, max_width: Option<u32>) -> Result<Self, String> {
        let layer = match layer {
            None => ScreenshotLayer::Mods,
            Some(text) if text.eq_ignore_ascii_case("game") => ScreenshotLayer::Game,
            Some(text) if text.eq_ignore_ascii_case("mods") => ScreenshotLayer::Mods,
            Some(text) => return Err(format!("layer must be \"game\" or \"mods\", not \"{text}\"")),
        };
        let max_width = max_width.unwrap_or(Self::DEFAULT_MAX_WIDTH);
        if !(Self::MIN_WIDTH..=Self::MAX_WIDTH).contains(&max_width) {
            return Err(format!("max_width must be from {} to {}, not {max_width}", Self::MIN_WIDTH, Self::MAX_WIDTH));
        }
        Ok(Self { layer, max_width })
    }

    pub fn scaled_size(&self, width: u32, height: u32) -> (u32, u32) {
        if width <= self.max_width {
            return (width, height);
        }
        let width = u64::from(width);
        let max = u64::from(self.max_width);
        let scaled = (u64::from(height) * max * 2 + width) / (width * 2);
        (self.max_width, scaled.max(1) as u32)
    }
}
