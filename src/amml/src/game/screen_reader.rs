use amml_api::Screen;
use headless_input_core::player::ScreenKind;

// Turns the top screen the loader reports (`Client::top_screen`) into a screen name, and a name into the kind of
// screen core tells apart. The names are the game's own names of those screens; the loader gives no other name, so
// a screen of no known kind has the empty name. A toast is drawn over whatever screen is under it, so it changes
// nothing: it gives the kind the last read gave
pub struct ScreenReader;

impl ScreenReader {
    // the HUD: the player is in a world with no menu open
    pub const HUD: &'static str = "hud_screen";
    // the HUD with the crosshair layer, shown after a dimension change
    pub const HUD_CROSSHAIR: &'static str = "hud_crosshair_screen";
    // the title screen
    pub const START: &'static str = "start_screen";
    // the Ore UI menus are one page named by its address, for example `ui://./hbui/index.html`
    pub const UI_PREFIX: &'static str = "ui://";
    // a loading or saving screen is named "..._progress_screen"
    pub const PROGRESS_MARK: &'static str = "progress_screen";
    // a notice (achievement, server message) over the screen under it
    pub const TOAST: &'static str = "toast_screen";

    // a loading screen of any kind
    pub const LOADING: &'static str = Self::PROGRESS_MARK;
    pub const PAUSE: &'static str = "pause_screen";
    // a screen that shows a container
    pub const CONTAINER: &'static str = "container_screen";

    // the name of the screen the loader reports; the empty name for `Screen::Other` and a screen this mod does not know
    pub fn name_of(screen: Screen) -> &'static str {
        match screen {
            Screen::Start => Self::START,
            Screen::Hud => Self::HUD,
            Screen::Loading => Self::LOADING,
            Screen::Toast => Self::TOAST,
            Screen::Pause => Self::PAUSE,
            Screen::Container => Self::CONTAINER,
            _ => "",
        }
    }

    // whether the name is the menus: the title screen, or an Ore UI page when the client has no local player. An
    // Ore UI page over a world is a screen opened over it
    pub fn is_title(name: &str, in_game: bool) -> bool {
        name == Self::START || (!in_game && name.starts_with(Self::UI_PREFIX))
    }

    // `in_game` is whether the client has a local player. Allocates only for a screen of no known kind, whose name it keeps, and for a toast over one
    pub fn kind(name: &str, previous: &ScreenKind, in_game: bool) -> ScreenKind {
        match name {
            Self::TOAST => previous.clone(),
            Self::HUD | Self::HUD_CROSSHAIR => ScreenKind::InWorld,
            Self::START => ScreenKind::Title,
            _ if Self::is_title(name, in_game) => ScreenKind::Title,
            _ if name.contains(Self::PROGRESS_MARK) => ScreenKind::Loading,
            _ => ScreenKind::Other(name.to_string()),
        }
    }
}
