use std::ffi::c_void;

use alaydriem_headless_input::join::JoinResultCallback;

// the slot order of the MSVC `_Func_base` the game uses: 0 `_Copy`, 1 `_Move`, 2 `_Do_call`, 3 `_Target_type`,
// 4 `_Delete_this`, 5 `_Get`
fn slots(block: &[*mut c_void; 8]) -> (*mut c_void, [*const c_void; 6]) {
    let object = block[7];
    // SAFETY: the block's last word is the pointer to the callable object, whose first qword is its vtable of six slots
    let vtable = unsafe { *object.cast::<*const [*const c_void; 6]>() };
    // SAFETY: as above
    (object, unsafe { *vtable })
}

#[test]
fn the_block_holds_a_callable_in_its_last_word_and_nothing_else() {
    let block = JoinResultCallback::function();

    assert_eq!(size_of_val(&block), JoinResultCallback::FUNCTION_SIZE);
    assert!(block[..7].iter().all(|word| word.is_null()));
    assert!(!block[7].is_null());
    assert_ne!(block[7], (&raw const block).cast_mut().cast::<c_void>());
}

#[test]
fn the_game_can_copy_move_call_and_delete_it() {
    let block = JoinResultCallback::function();
    let (object, slots) = slots(&block);
    let mut place = [0u8; 64];
    let place = place.as_mut_ptr().cast::<c_void>();

    // SAFETY: the slots have the `_Func_base` signatures; the object is the static callable
    unsafe {
        let copy: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void = std::mem::transmute(slots[0]);
        assert_eq!(copy(object, place), object);
        let moved: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void = std::mem::transmute(slots[1]);
        assert_eq!(moved(object, place), object);
        let call: unsafe extern "C" fn(*mut c_void, *const i32) = std::mem::transmute(slots[2]);
        let code = 8i32;
        call(object, &code);
        let target: unsafe extern "C" fn(*mut c_void) -> *const c_void = std::mem::transmute(slots[3]);
        assert!(!target(object).is_null());
        let delete: unsafe extern "C" fn(*mut c_void, bool) = std::mem::transmute(slots[4]);
        delete(object, true);
        delete(object, false);
        let get: unsafe extern "C" fn(*mut c_void) -> *mut c_void = std::mem::transmute(slots[5]);
        assert_eq!(get(object), object);
    }
}

#[test]
fn every_block_shares_the_one_static_callable() {
    assert_eq!(JoinResultCallback::function()[7], JoinResultCallback::function()[7]);
}

#[test]
fn the_target_type_is_an_empty_type_info_with_a_nul_decorated_name() {
    let block = JoinResultCallback::function();
    let (object, slots) = slots(&block);

    // SAFETY: the slot has the `_Target_type` signature; the object is the static callable
    let target = unsafe {
        let target: unsafe extern "C" fn(*mut c_void) -> *const c_void = std::mem::transmute(slots[3]);
        target(object).cast::<u8>()
    };

    // SAFETY: the block is `TARGET_TYPE_SIZE` bytes of static memory
    let bytes = unsafe { std::slice::from_raw_parts(target, JoinResultCallback::TARGET_TYPE_SIZE) };
    assert_eq!(bytes[0x10], 0);
    assert_eq!(bytes[0x11], 0);
}

#[test]
fn the_last_result_code_is_taken_once_and_a_clear_drops_it() {
    let block = JoinResultCallback::function();
    let (object, slots) = slots(&block);
    assert_eq!(JoinResultCallback::take_result(), None);

    // SAFETY: the slot has the `_Do_call` signature; the object is the static callable
    unsafe {
        let call: unsafe extern "C" fn(*mut c_void, *const i32) = std::mem::transmute(slots[2]);
        call(object, &8);
        call(object, &9);
    }
    assert_eq!(JoinResultCallback::take_result(), Some(9));
    assert_eq!(JoinResultCallback::take_result(), None);

    // SAFETY: as above
    unsafe {
        let call: unsafe extern "C" fn(*mut c_void, *const i32) = std::mem::transmute(slots[2]);
        call(object, &8);
    }
    JoinResultCallback::clear_result();
    assert_eq!(JoinResultCallback::take_result(), None);
}

#[test]
fn a_null_result_pointer_is_taken_as_no_code_the_game_gave() {
    let block = JoinResultCallback::function();
    let (object, slots) = slots(&block);

    // SAFETY: the slot has the `_Do_call` signature; the object is the static callable; a null result is allowed
    unsafe {
        let call: unsafe extern "C" fn(*mut c_void, *const i32) = std::mem::transmute(slots[2]);
        call(object, std::ptr::null());
    }

    assert_eq!(JoinResultCallback::take_result(), Some(-1));
}
