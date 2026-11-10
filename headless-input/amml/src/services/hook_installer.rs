use amml_sdk::{HookHandle, SdkError};

use crate::services::HeadlessRuntime;
use crate::traits::GameHost;
use crate::types::HookSpec;

// installs the headless-input mod's hooks: every required entry must resolve before any hook goes in; a missing optional
// entry turns off only its own feature, with one log line
pub struct HookInstaller;

impl HookInstaller {
    pub fn install<H: GameHost>(host: &H, runtime: &HeadlessRuntime, specs: &[HookSpec]) -> Result<(), SdkError> {
        let mut targets = Vec::with_capacity(specs.len());
        for spec in specs {
            match host.resolve(spec.entry) {
                Ok(target) => targets.push((target, spec)),
                Err(error) if spec.required => return Err(error),
                Err(error) => {
                    host.warn(&format!("{} did not resolve ({error}); {} is off", spec.entry, spec.feature));
                }
            }
        }
        for (target, spec) in targets {
            let kept = match host.hook(target, spec) {
                Ok(hook) => Self::keep(host, runtime, hook),
                Err(error) => Err(error),
            };
            if let Err(error) = kept {
                Self::remove_all(host, runtime);
                return Err(error);
            }
        }
        Ok(())
    }

    pub fn remove_all<H: GameHost>(host: &H, runtime: &HeadlessRuntime) {
        for hook in runtime.take_hooks() {
            if let Err(error) = host.unhook(hook) {
                host.warn(&format!("unhook failed: {error}"));
            }
        }
    }

    // a hook the runtime cannot keep is removed again, so no installed hook is left without a handle
    fn keep<H: GameHost>(host: &H, runtime: &HeadlessRuntime, hook: HookHandle) -> Result<(), SdkError> {
        if runtime.keep_hook(hook) {
            return Ok(());
        }
        let _ = host.unhook(hook);
        Err(SdkError::Mod(format!("more than {} hooks", HeadlessRuntime::HOOK_CAPACITY)))
    }
}
