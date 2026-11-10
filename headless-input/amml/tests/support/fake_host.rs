use std::cell::RefCell;
use std::collections::BTreeMap;

use alaydriem_headless_input::traits::GameHost;
use alaydriem_headless_input::types::HookSpec;
use amml_sdk::{HookHandle, SdkError};

// a loader stand-in: entries it knows resolve, the rest are missing; hooks are recorded, never installed
pub struct FakeHost {
    entries: BTreeMap<&'static str, u64>,
    hooked: RefCell<Vec<&'static str>>,
    warnings: RefCell<Vec<String>>,
}

impl FakeHost {
    pub fn new(entries: &[(&'static str, u64)]) -> Self {
        Self {
            entries: entries.iter().copied().collect(),
            hooked: RefCell::new(Vec::new()),
            warnings: RefCell::new(Vec::new()),
        }
    }

    pub fn hooked(&self) -> Vec<&'static str> {
        self.hooked.borrow().clone()
    }

    pub fn warnings(&self) -> Vec<String> {
        self.warnings.borrow().clone()
    }
}

impl GameHost for FakeHost {
    fn resolve(&self, id: &str) -> Result<u64, SdkError> {
        self.entries.get(id).copied().ok_or_else(|| SdkError::MissingEntry(id.to_string()))
    }

    fn hook(&self, _target: u64, spec: &HookSpec) -> Result<HookHandle, SdkError> {
        let mut hooked = self.hooked.borrow_mut();
        hooked.push(spec.entry);
        Ok(HookHandle(hooked.len() as u64))
    }

    fn unhook(&self, _hook: HookHandle) -> Result<(), SdkError> {
        Ok(())
    }

    fn warn(&self, message: &str) {
        self.warnings.borrow_mut().push(message.to_string());
    }
}
