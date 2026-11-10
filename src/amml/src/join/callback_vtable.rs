use std::ffi::c_void;

// the six virtual functions of the MSVC `_Func_base` in slot order, as the game's own callables lay them out
#[repr(C)]
pub(crate) struct CallbackVtable {
    // `_Copy(where)`: gives the callable the copy lives in
    pub copy: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void,
    // `_Move(where)`
    pub moved: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void,
    // `_Do_call(JoinServerWorldResult&&)`
    pub do_call: unsafe extern "C" fn(*mut c_void, *const i32),
    // `_Target_type`
    pub target_type: unsafe extern "C" fn(*mut c_void) -> *const c_void,
    // `_Delete_this(bool dealloc)`
    pub delete_this: unsafe extern "C" fn(*mut c_void, bool),
    // `_Get`
    pub get: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
}
