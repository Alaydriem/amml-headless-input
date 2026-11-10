use std::io;
use std::path::{Path, PathBuf};

use super::{LegacyCopy, TokenStore};

// The one-time copy of `mcp.tokens.json` from the folder older builds used, `%LOCALAPPDATA%\amml\headless-input`, into
// the shared data folder the loader gives the mod. The copy is made only when the new folder holds no token file and the
// old one exists, so the clients paired before keep their tokens. The bytes go to a temporary file first, which then
// takes the new name only when no other instance made the file in the meantime. The old file is never changed
pub struct LegacyTokenFile;

impl LegacyTokenFile {
    // `<local_app_data>/amml/headless-input`, the folder of the token file in older builds
    pub fn folder_in(local_app_data: &Path) -> PathBuf {
        local_app_data.join("amml").join("headless-input")
    }

    // copies `<legacy>/mcp.tokens.json` to `<folder>/mcp.tokens.json` when only the first exists
    pub fn copy_once(legacy: &Path, folder: &Path) -> io::Result<LegacyCopy> {
        let to = folder.join(TokenStore::FILE_NAME);
        if to.exists() {
            return Ok(LegacyCopy::AlreadyPresent);
        }
        let from = legacy.join(TokenStore::FILE_NAME);
        if !from.is_file() {
            return Ok(LegacyCopy::NoLegacyFile);
        }
        std::fs::create_dir_all(folder)?;
        let temp = to.with_extension(format!("json.{}.legacy.tmp", std::process::id()));
        let copied = std::fs::copy(&from, &temp).and_then(|_| std::fs::hard_link(&temp, &to));
        let _ = std::fs::remove_file(&temp);
        match copied {
            Ok(()) => Ok(LegacyCopy::Copied),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(LegacyCopy::AlreadyPresent),
            Err(error) => Err(error),
        }
    }
}
