use std::ffi::c_void;

use amml_game::{ClientInstance, EntryId, GameEntries, GameHeap, RawObject};
use amml_msvc::{MsvcString, SharedPtr};

use crate::command::CommandClient;
use crate::game::{GameStrings, HeadlessEntry};

// The game's client behind the `CommandClient` trait. It builds the CommandRequestPacket the chat builds, puts the text
// in it and sends it through the client's packet sender; the packet then goes away with the last reference. A call whose
// entry is off does nothing and answers no. Game thread
pub struct GameCommandClient<'a> {
    client: ClientInstance<'a>,
    entries: &'a GameEntries,
}

impl<'a> GameCommandClient<'a> {
    // `MinecraftPacketIds::CommandRequestPacket`, the id `MinecraftPackets::createPacket` takes to make the packet
    pub const COMMAND_REQUEST_PACKET_ID: u32 = 0x4D;

    pub fn new(client: ClientInstance<'a>, entries: &'a GameEntries) -> Self {
        Self { client, entries }
    }

    // the code address of a function entry; None when the entry is off or 0, which is no function
    fn function_address(&self, id: EntryId) -> Option<usize> {
        usize::try_from(self.entries.get(id)?).ok().filter(|address| *address != 0)
    }

    // the packet sender of the client: `PacketSender& ClientInstance::getPacketSender()` is a slot of the client
    #[allow(unsafe_code)]
    fn packet_sender(&self) -> Option<*mut c_void> {
        let slot = self.entries.get(HeadlessEntry::CLIENT_INSTANCE_GET_PACKET_SENDER)?;
        // SAFETY: the wrapper holds a live ClientInstance, whose first qword is its vtable, and the entry is a slot
        // of it
        let getter = unsafe { RawObject::slot(self.client.as_ptr(), slot) }?;
        // SAFETY: the slot holds `PacketSender& ClientInstance::getPacketSender()`, which returns its address in rax,
        // and the client is live
        unsafe { RawObject::call_pointer(getter, self.client.as_ptr()) }
    }

    // the `send` function of the sender: `void PacketSender::send(Packet&)` is a slot of its vtable
    #[allow(unsafe_code)]
    fn send_function(&self, sender: *mut c_void) -> Option<*const c_void> {
        let slot = self.entries.get(HeadlessEntry::PACKET_SENDER_SEND)?;
        // SAFETY: the sender is a live PacketSender, whose first qword is its vtable, and the entry is a slot of it
        unsafe { RawObject::slot(sender, slot) }
    }

    // a default CommandRequestPacket from the game's packet factory; None when the factory made none. The caller owns
    // the one use of the returned pointer and must release it
    #[allow(unsafe_code)]
    fn create_packet(factory: usize) -> Option<(SharedPtr<c_void>, *mut c_void)> {
        // SAFETY: an all-zero shared_ptr is the empty one
        let mut made: SharedPtr<c_void> = unsafe { std::mem::zeroed() };
        // SAFETY: the entry is `std::shared_ptr<Packet> MinecraftPackets::createPacket(MinecraftPacketIds)`, a static
        // function: the hidden return buffer in rcx and the id in edx. It builds the packet in the buffer
        let create: unsafe extern "C" fn(*mut SharedPtr<c_void>, u32) -> *mut SharedPtr<c_void> =
            unsafe { std::mem::transmute(factory) };
        // SAFETY: `made` is 16 writable bytes, the game thread is the caller's, and the id is a packet id the game
        // knows
        unsafe { create(&raw mut made, Self::COMMAND_REQUEST_PACKET_ID) };
        let packet = made.get().map(|packet| std::ptr::from_ref(packet).cast_mut())?;
        Some((made, packet))
    }
}

impl CommandClient for GameCommandClient<'_> {
    fn has_local_player(&self) -> bool {
        self.client.local_player().is_some()
    }

    // The packet is built, filled and sent in one call. Everything the send needs is looked up before the packet
    // exists, so a missing step leaves nothing to undo but the text. The text is written over the empty `mCommand` the
    // factory left, so nothing is freed first, and the packet owns the text from then on
    #[allow(unsafe_code)]
    fn send_command(&self, text: &str) -> bool {
        let Some(factory) = self.function_address(HeadlessEntry::MINECRAFT_PACKETS_CREATE_PACKET) else {
            return false;
        };
        let Some(command_at) = self.entries.get(HeadlessEntry::COMMAND_REQUEST_PACKET_M_COMMAND) else {
            return false;
        };
        let Some(size) = self.entries.get(HeadlessEntry::COMMAND_REQUEST_PACKET_SIZE) else {
            return false;
        };
        // the string must lie inside the packet
        let string_size = std::mem::size_of::<MsvcString>() as u64;
        if command_at.checked_add(string_size).is_none_or(|end| end > size) {
            return false;
        }
        let Ok(command_at) = usize::try_from(command_at) else {
            return false;
        };
        let Some(heap) = GameHeap::msvc() else {
            return false;
        };
        let Some(sender) = self.packet_sender() else {
            return false;
        };
        let Some(send) = self.send_function(sender) else {
            return false;
        };
        let Some(mut string) = GameStrings::heap_string(&heap, text) else {
            return false;
        };
        let Some((mut made, packet)) = Self::create_packet(factory) else {
            // SAFETY: the string came from `GameStrings::heap_string` with this heap and is not used again
            unsafe { string.release(&heap) };
            return false;
        };
        // SAFETY: `packet` is the live CommandRequestPacket the factory made, `command_at` is the offset of its
        // `mCommand`, which holds an empty small string with no block to free, and the string lies inside the
        // `CommandRequestPacket.size` bytes, as checked above. The string moves into the packet, which frees its
        // block in its destructor, so it is not released here
        unsafe { packet.cast::<u8>().add(command_at).cast::<MsvcString>().write_unaligned(string) };
        // SAFETY: the entry is a slot holding `void PacketSender::send(Packet&)`: this in rcx and the packet in rdx.
        // The sender and the packet are live, and the call does not take the packet
        let call: unsafe extern "C" fn(*mut c_void, *mut c_void) = unsafe { std::mem::transmute(send) };
        // SAFETY: as above, on the game thread
        unsafe { call(sender, packet) };
        // SAFETY: `made` holds the one use the factory gave; the last use runs the packet's destructor, which frees
        // the text
        unsafe { made.release() };
        true
    }
}
