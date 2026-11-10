use headless_input_core::player::ScreenKind;

use crate::types::FrameView;

// The position, angles and dimension of the last frame in this world that read them all. A frame whose read failed
// takes them in place of the zero position and unknown dimension it was read with. A frame outside a world drops
// them, so a new world never starts from the last one's values; its dimension then stays unknown until it is read.
// It also keeps the screen of the last frame, which a toast on top gives again; the title before any frame
pub struct LastView {
    held: Option<([f32; 3], f32, f32, Option<i32>)>,
    screen: ScreenKind,
}

impl LastView {
    pub fn new() -> Self {
        Self { held: None, screen: ScreenKind::Title }
    }

    // keeps the fields of a fresh frame; gives a frame that is not fresh the kept fields, when there are any. The
    // screen and `in_world` of the frame stay as read
    pub fn settle(&mut self, frame: &mut FrameView) {
        if self.screen != frame.view.screen {
            self.screen = frame.view.screen.clone();
        }
        let view = &mut frame.view;
        if !frame.in_world {
            self.held = None;
        } else if frame.fresh {
            self.held = Some((view.position, view.yaw, view.pitch, view.dimension));
        } else if let Some((position, yaw, pitch, dimension)) = self.held {
            view.position = position;
            view.yaw = yaw;
            view.pitch = pitch;
            view.dimension = dimension;
        }
    }

    pub fn screen(&self) -> &ScreenKind {
        &self.screen
    }

    pub fn clear(&mut self) {
        self.held = None;
        self.screen = ScreenKind::Title;
    }
}

impl Default for LastView {
    fn default() -> Self {
        Self::new()
    }
}
