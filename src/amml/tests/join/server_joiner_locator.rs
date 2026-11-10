use std::cell::Cell;
use std::ffi::c_void;

use alaydriem_headless_input::join::ServerJoinerLocator;

const OWNER_SLOT: u64 = 3;
const JOINER_AT: u64 = 0x80;
const MEMBER_AT: u64 = 0x268;

// a MinecraftGame stand-in: the first qword is a vtable whose slot 3 is the owner getter
#[repr(C)]
struct FakeGame {
    vtable: *const *const c_void,
}

// an owner object: the NetworkWorldJoiner pointer sits at JOINER_AT
#[repr(C)]
struct FakeOwner {
    pad: [u64; 16],
    joiner: *mut c_void,
}

thread_local! {
    static OWNER: Cell<*mut c_void> = const { Cell::new(std::ptr::null_mut()) };
}

unsafe extern "C" fn owner_getter(_this: *mut c_void) -> *mut c_void {
    OWNER.with(Cell::get)
}

fn locate(game: &mut FakeGame) -> Option<*mut c_void> {
    // SAFETY: the fake game has a vtable with the getter in slot 3, and the owner it returns is a FakeOwner or null
    unsafe { ServerJoinerLocator::locate((game as *mut FakeGame).cast(), OWNER_SLOT, JOINER_AT, MEMBER_AT) }
}

fn game() -> (FakeGame, Box<[*const c_void; 8]>) {
    let mut table: Box<[*const c_void; 8]> = Box::new([std::ptr::null(); 8]);
    table[OWNER_SLOT as usize] = owner_getter as unsafe extern "C" fn(*mut c_void) -> *mut c_void as *const c_void;
    (FakeGame { vtable: table.as_ptr() }, table)
}

#[test]
fn the_joiner_is_a_member_of_the_network_world_joiner() {
    let (mut game, _table) = game();
    let network_joiner = Box::new([0u8; 0x300]);
    let owner = Box::new(FakeOwner { pad: [0; 16], joiner: network_joiner.as_ptr().cast_mut().cast() });
    OWNER.with(|cell| cell.set((&raw const *owner).cast_mut().cast()));

    let found = locate(&mut game).unwrap();

    assert_eq!(found as usize, network_joiner.as_ptr() as usize + MEMBER_AT as usize);
}

#[test]
fn a_null_owner_gives_no_joiner() {
    let (mut game, _table) = game();
    OWNER.with(|cell| cell.set(std::ptr::null_mut()));

    assert!(locate(&mut game).is_none());
}

#[test]
fn a_null_network_world_joiner_gives_no_joiner() {
    let (mut game, _table) = game();
    let owner = Box::new(FakeOwner { pad: [0; 16], joiner: std::ptr::null_mut() });
    OWNER.with(|cell| cell.set((&raw const *owner).cast_mut().cast()));

    assert!(locate(&mut game).is_none());
}

#[test]
fn a_null_game_or_an_empty_slot_gives_no_joiner() {
    // SAFETY: a null game is refused before any read
    assert!(unsafe { ServerJoinerLocator::locate(std::ptr::null_mut(), OWNER_SLOT, JOINER_AT, MEMBER_AT) }.is_none());
    let (mut game, mut table) = game();
    table[OWNER_SLOT as usize] = std::ptr::null();

    assert!(locate(&mut game).is_none());
}
