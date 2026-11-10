use crate::join::callback_vtable::CallbackVtable;

// the callable: only its vtable pointer
#[repr(C)]
pub(crate) struct CallbackObject {
    pub vtable: *const CallbackVtable,
}

// SAFETY: the object holds a pointer to an immutable static vtable and is never written
#[allow(unsafe_code)]
unsafe impl Sync for CallbackObject {}
