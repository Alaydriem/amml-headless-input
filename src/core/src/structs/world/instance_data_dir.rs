use std::path::{Path, PathBuf};

use hcl::Expression;

// Reads the `data_dir` rule of the loader file `instance.hcl`.
pub struct InstanceDataDir;

impl InstanceDataDir {
    // A string `data_dir`; a relative one is joined to `game`. Anything else gives `None`.
    pub fn parse(text: &str, game: &Path) -> Option<PathBuf> {
        let body = hcl::parse(text).ok()?;
        let attribute = body.attributes().find(|a| a.key() == "data_dir")?;
        match attribute.expr() {
            Expression::String(value) => {
                let dir = PathBuf::from(value);
                Some(if dir.is_absolute() { dir } else { game.join(dir) })
            }
            _ => None,
        }
    }
}
