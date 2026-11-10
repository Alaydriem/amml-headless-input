// The game functions the headless-input mod hooks. Each body calls the original, then one HeadlessRuntime method

use amml_sdk::amml_detour;

use crate::services::HeadlessRuntime;

// bool ClientInstance::update(bool isInitFinished) (26.60: update(bool, bool)): once per frame on the game thread
// The third argument is carried as a raw u64 so the register reaches the original bit for bit
amml_detour!(pub mod client_update(this: usize, init_finished: u8, extra: u64) -> u8 |original| {
    let result = original(this, init_finished, extra);
    if let Some(runtime) = HeadlessRuntime::get() {
        // SAFETY: `this` is the live ClientInstance the game passed to its update, on the game thread
        #[allow(unsafe_code)]
        unsafe {
            runtime.tick(this);
        }
    }
    result
});

// void ClientInstance::onTick(int nTick, int maxTick): once per client tick on the game thread, in menus too. rcx is the
// GameCallbacks subobject of the ClientInstance, not the ClientInstance itself
amml_detour!(pub mod client_on_tick(callbacks: usize, tick: i32, max_tick: i32) -> () |original| {
    original(callbacks, tick, max_tick);
    if let Some(runtime) = HeadlessRuntime::get() {
        // SAFETY: `callbacks` is the live GameCallbacks subobject the game passed to its tick, on the game thread
        #[allow(unsafe_code)]
        unsafe {
            runtime.tick_game(callbacks);
        }
    }
});
