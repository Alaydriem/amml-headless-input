use std::path::PathBuf;

use amml_api::Dirs;

// The folders the runtime uses, from the loader's mod folders. `instance_dir` is the AMML instance folder;
// `game_data_dir` the folder the game keeps its worlds under, when the loader gives one
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeFolders {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub shared_data_dir: Option<PathBuf>,
    pub instance_dir: Option<PathBuf>,
    pub game_data_dir: Option<PathBuf>,
}

impl RuntimeFolders {
    // the folders of `dirs`; a folder the loader does not give is empty
    pub fn from_dirs(dirs: &Dirs) -> Self {
        Self {
            config_dir: dirs.config_dir().unwrap_or_default(),
            data_dir: dirs.data_dir().unwrap_or_default(),
            shared_data_dir: dirs.shared_data_dir(),
            instance_dir: dirs.instance_dir(),
            game_data_dir: dirs.game_data_dir(),
        }
    }
}
