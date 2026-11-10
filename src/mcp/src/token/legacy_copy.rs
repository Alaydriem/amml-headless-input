/// What `LegacyTokenFile::copy_once` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyCopy {
    // the old token file is now the token file of the new folder
    Copied,
    // the new folder holds a token file already; the old file is not read
    AlreadyPresent,
    // the old folder holds no token file
    NoLegacyFile,
}
