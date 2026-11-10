use std::path::PathBuf;

use amm_client::test_support::FakeTable;
use amml_sdk::ModDirs;

// the folders of a mod installed in the AMML instance `INSTANCE`
pub struct RuntimeDirs;

impl RuntimeDirs {
    pub const INSTANCE: &'static str = "1.26.62.01";
    pub const DATA_DIR: &'static str = r"C:\mods\headless-input\data";
    pub const SHARED_DATA_DIR: &'static str = r"C:\Users\me\AppData\Local\amml\mod-data\@alaydriem\headless-input";

    pub fn dirs() -> ModDirs {
        let mod_dir =
            format!(r"C:\Users\me\AppData\Local\amml\instances\{}\mods\amml\@alaydriem\headless-input", Self::INSTANCE);
        ModDirs {
            mod_dir: PathBuf::from(mod_dir),
            config_dir: PathBuf::from(FakeTable::CONFIG_DIR),
            data_dir: PathBuf::from(Self::DATA_DIR),
            shared_data_dir: Some(PathBuf::from(Self::SHARED_DATA_DIR)),
        }
    }
}
