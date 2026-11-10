use std::ffi::c_void;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicI32, Ordering};

use crate::join::callback_object::CallbackObject;
use crate::join::callback_vtable::CallbackVtable;

// The `std::function<void(World::JoinServerWorldResult)>` the game's `joinExternalServer` takes by value. The game
// calls the callback on error paths without an emptiness check, so an empty one throws; this one is never empty. Its
// callable lives in static memory and its calls store the result code and log it
pub struct JoinResultCallback;

static VTABLE: CallbackVtable = CallbackVtable {
    copy: JoinResultCallback::copy,
    moved: JoinResultCallback::moved,
    do_call: JoinResultCallback::do_call,
    target_type: JoinResultCallback::target_type,
    delete_this: JoinResultCallback::delete_this,
    get: JoinResultCallback::get,
};

static OBJECT: CallbackObject = CallbackObject { vtable: &VTABLE };

// the identity the target type query gives: an empty MSVC `type_info`, whose vtable and spare words are zero and whose
// decorated name is an empty string at +0x10
static TARGET_TYPE: [u8; JoinResultCallback::TARGET_TYPE_SIZE] = [0; JoinResultCallback::TARGET_TYPE_SIZE];

// the empty decorated name at +0x10 and +0x11 must be inside the block
const _: () = assert!(JoinResultCallback::TARGET_TYPE_SIZE >= 0x20);

// the last result code the game gave, or `NO_RESULT`. The callback runs on a worker thread and the join service reads it
// on the game thread. The code is a status on its own and guards no other data, so Relaxed ordering is enough
static LAST_RESULT: AtomicI32 = AtomicI32::new(JoinResultCallback::NO_RESULT);

#[allow(unsafe_code)]
impl JoinResultCallback {
    // `sizeof(std::function)` under MSVC: seven words of small-object space and the callable pointer
    pub const FUNCTION_SIZE: usize = 0x40;

    // the bytes of the `type_info` block the target type query gives: vtable, spare word and the name at +0x10
    pub const TARGET_TYPE_SIZE: usize = 0x20;

    // marks that no code was stored; the game never passes it
    const NO_RESULT: i32 = i32::MIN;

    // the result code stored since the last take or clear, taken once
    pub fn take_result() -> Option<i32> {
        match LAST_RESULT.swap(Self::NO_RESULT, Ordering::Relaxed) {
            Self::NO_RESULT => None,
            code => Some(code),
        }
    }

    // drops a stored result code
    pub fn clear_result() {
        LAST_RESULT.store(Self::NO_RESULT, Ordering::Relaxed);
    }

    // a new std::function block that points to the static callable. The callable is not in the block, so the game
    // treats it as a heap callable: it steals the pointer, copies it through `_Copy` and ends with `_Delete_this`
    pub fn function() -> [*mut c_void; 8] {
        let mut block = [std::ptr::null_mut(); 8];
        block[7] = (&raw const OBJECT).cast_mut().cast();
        block
    }

    // the static callable has no state: a copy or a move is itself
    pub(crate) unsafe extern "C" fn copy(this: *mut c_void, _place: *mut c_void) -> *mut c_void {
        this
    }

    pub(crate) unsafe extern "C" fn moved(this: *mut c_void, _place: *mut c_void) -> *mut c_void {
        this
    }

    // may run on a worker thread: it stores and logs the code and does nothing else. A panic in it ends the process, as no frame
    // of ours may unwind into the game
    pub(crate) unsafe extern "C" fn do_call(_this: *mut c_void, result: *const i32) {
        let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| {
            // SAFETY: the game passes a pointer to the result code; a null pointer is logged as no code
            let code = if result.is_null() { -1 } else { unsafe { *result } };
            LAST_RESULT.store(code, Ordering::Relaxed);
            curia::info!("the game answered the join", { result: code });
        }));
        if outcome.is_err() {
            std::process::abort();
        }
    }

    pub(crate) unsafe extern "C" fn target_type(_this: *mut c_void) -> *const c_void {
        TARGET_TYPE.as_ptr().cast()
    }

    // the callable is static: nothing is freed
    pub(crate) unsafe extern "C" fn delete_this(_this: *mut c_void, _dealloc: bool) {}

    pub(crate) unsafe extern "C" fn get(this: *mut c_void) -> *mut c_void {
        this
    }
}
