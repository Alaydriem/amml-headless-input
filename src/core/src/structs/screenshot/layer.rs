#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenshotLayer {
    // the world without mod overlays
    Game,
    // the world with mod overlays
    Mods,
}
