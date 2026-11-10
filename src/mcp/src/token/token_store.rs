use std::fmt::Write as _;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use subtle::ConstantTimeEq;

use super::{IssuedToken, TokenCache, TokenFile};

// The bearer tokens, kept in `mcp.tokens.json`. Every game instance of the user shares the file, so a token issued by
// one hub opens whichever instance is hub later. Each token is 32 random bytes as 64 lowercase hex characters. Two
// kinds share the file: pairing tokens, at most `CAPACITY` (a new one past it drops the oldest pairing token), and
// the one member token the instances present to the hub (made only when the file holds none). A member token never
// drops a pairing token.
#[derive(Debug)]
pub struct TokenStore {
    file: PathBuf,
    cache: Mutex<TokenCache>,
}

impl TokenStore {
    pub const FILE_NAME: &'static str = "mcp.tokens.json";
    // pairing tokens; the member token is not counted
    pub const CAPACITY: usize = 32;
    // the client name of the member token
    pub const MEMBER_CLIENT: &'static str = "headless-input-member";

    // reads `<folder>/mcp.tokens.json`. A missing or broken file gives no tokens; nothing is written here
    pub fn load(folder: &Path) -> TokenStore {
        let file = folder.join(Self::FILE_NAME);
        let cache = Self::read(&file);
        Self { file, cache: Mutex::new(cache) }
    }

    // makes a new pairing token for `client`, saves it and returns it. The file is read first, so a token another
    // instance saved is kept
    pub fn issue(&self, client: &str) -> io::Result<String> {
        self.save(client, false)
    }

    // the token members present to the hub: the saved member token, or a new one when the file holds none
    pub fn member_token(&self) -> io::Result<String> {
        self.save(Self::MEMBER_CLIENT, true)
    }

    fn save(&self, client: &str, member: bool) -> io::Result<String> {
        let mut cache = self.lock();
        *cache = Self::read(&self.file);
        if member && let Some(issued) = cache.tokens.iter().rev().find(|issued| issued.member) {
            return Ok(issued.token.clone());
        }
        let token = Self::generate()?;
        let issued_ms = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_millis() as u64);
        let mut tokens = cache.tokens.clone();
        if !member {
            while tokens.iter().filter(|issued| !issued.member).count() >= Self::CAPACITY {
                match tokens.iter().position(|issued| !issued.member) {
                    Some(oldest) => tokens.remove(oldest),
                    None => break,
                };
            }
        }
        tokens.push(IssuedToken { token: token.clone(), client: client.to_owned(), issued_ms, member });
        self.write(&tokens)?;
        *cache = TokenCache::new(tokens);
        Ok(token)
    }

    // true when `token` is one of the stored tokens. Each stored token is compared in constant time, and every one is
    // compared, so the time does not tell which one matched. The file is read on every check, so a token another
    // instance issued opens this one and a revoke in another instance closes it, even when the file time did not change
    pub fn contains(&self, token: &str) -> bool {
        let mut cache = self.lock();
        *cache = Self::read(&self.file);
        Self::matches(&cache.tokens, token)
    }

    // the number of stored tokens of both kinds
    pub fn len(&self) -> usize {
        self.lock().tokens.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // removes every token, in memory and in the file. Every client must pair again
    pub fn revoke_all(&self) -> io::Result<()> {
        let mut cache = self.lock();
        self.write(&[])?;
        *cache = TokenCache::new(Vec::new());
        Ok(())
    }

    fn matches(tokens: &[IssuedToken], token: &str) -> bool {
        let given = token.as_bytes();
        let mut found = 0u8;
        for issued in tokens {
            found |= issued.token.as_bytes().ct_eq(given).unwrap_u8();
        }
        found == 1 && !given.is_empty()
    }

    fn read(file: &Path) -> TokenCache {
        let tokens = std::fs::read_to_string(file)
            .ok()
            .and_then(|text| serde_json::from_str::<TokenFile>(&text).ok())
            .map(|parsed| parsed.tokens)
            .unwrap_or_default();
        TokenCache::new(tokens)
    }

    // writes a temp file and renames it, so a crash never leaves a broken file
    fn write(&self, tokens: &[IssuedToken]) -> io::Result<()> {
        if let Some(dir) = self.file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = serde_json::to_string_pretty(&TokenFile { tokens: tokens.to_vec() }).map_err(io::Error::other)?;
        let temp = self.file.with_extension(format!("json.{}.tmp", std::process::id()));
        std::fs::write(&temp, text)?;
        std::fs::rename(&temp, &self.file)
    }

    fn generate() -> io::Result<String> {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(|error| io::Error::other(error.to_string()))?;
        let mut token = String::with_capacity(64);
        for byte in bytes {
            let _ = write!(token, "{byte:02x}");
        }
        Ok(token)
    }

    fn lock(&self) -> MutexGuard<'_, TokenCache> {
        self.cache.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
