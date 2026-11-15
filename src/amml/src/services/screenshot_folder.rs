use std::fs;
use std::path::PathBuf;

use headless_input_core::structs::ScreenshotRetention;

// the folder the screenshots are written to; it keeps at most ScreenshotRetention::MAX_SCREENSHOTS of them
pub struct ScreenshotFolder {
    dir: PathBuf,
}

impl ScreenshotFolder {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    // the path of the screenshot made at `unix_millis`; makes the folder when it is missing
    pub fn next_path(&self, unix_millis: u64) -> PathBuf {
        if let Err(error) = fs::create_dir_all(&self.dir) {
            curia::warn!("the screenshot folder could not be made", {
                folder: self.dir.display().to_string(),
                error: error.to_string()
            });
        }
        self.dir.join(ScreenshotRetention::file_name(unix_millis))
    }

    // deletes the oldest screenshots past the retention bound; other files stay
    pub fn prune(&self) {
        let Ok(entries) = fs::read_dir(&self.dir) else { return };
        let names: Vec<String> =
            entries.filter_map(Result::ok).filter_map(|entry| entry.file_name().into_string().ok()).collect();
        for name in ScreenshotRetention::to_delete(&names) {
            if let Err(error) = fs::remove_file(self.dir.join(&name)) {
                curia::warn!("an old screenshot could not be deleted", { file: name, error: error.to_string() });
            }
        }
    }
}
