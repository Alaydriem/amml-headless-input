use std::cell::{Cell, RefCell};

use alaydriem_headless_input::errors::{LeaseError, OutputError};
use alaydriem_headless_input::traits::ManagerPort;
use amm_client::link::PointerOp;
use amml_api::MoveIntent;

// one call the output side made on the manager
#[derive(Debug, Clone, PartialEq)]
pub enum Call {
    Acquire,
    Release,
    Key(u32, bool),
    Pointer(PointerOp),
    Move(MoveIntent),
    // a hotbar select through the game HUD, slot 0 to 8
    Hotbar(u8),
    // a step outside the manager the test wrote into the record, to check the order
    Mark(&'static str),
}

// a manager stand-in: every call is recorded in order. A lease is granted unless a holder is set. While the panel
// is open every send is refused; while another mod holds the input every down is refused and ups are taken. The zero
// move is an up. A manager made `without_move` answers every move `Unsupported`; `fail_moves` and `fail_keys` set the
// answer to every non-zero move and every key down
pub struct FakeManager {
    calls: RefCell<Vec<Call>>,
    holder: Option<String>,
    panel: Cell<bool>,
    taken: Cell<bool>,
    move_supported: bool,
    move_error: RefCell<Option<OutputError>>,
    key_error: RefCell<Option<OutputError>>,
}

impl FakeManager {
    pub fn new() -> Self {
        Self {
            calls: RefCell::new(Vec::new()),
            holder: None,
            panel: Cell::new(false),
            taken: Cell::new(false),
            move_supported: true,
            move_error: RefCell::new(None),
            key_error: RefCell::new(None),
        }
    }

    // a manager older than the move command
    pub fn without_move() -> Self {
        Self { move_supported: false, ..Self::new() }
    }

    // from now on every non-zero move is answered with `error`
    pub fn fail_moves(&self, error: OutputError) {
        *self.move_error.borrow_mut() = Some(error);
    }

    // from now on every key down is answered with `error`
    pub fn fail_keys(&self, error: OutputError) {
        *self.key_error.borrow_mut() = Some(error);
    }

    // a manager whose Input lease `holder` holds
    pub fn held_by(holder: &str) -> Self {
        Self { holder: Some(holder.to_string()), ..Self::new() }
    }

    // from now on every send is refused, as the manager does while its panel is open
    pub fn open_panel(&self) {
        self.panel.set(true);
    }

    // from now on another mod holds the input: downs are refused, ups are taken
    pub fn take_lease(&self) {
        self.taken.set(true);
    }

    // writes a step that is not a manager call into the record
    pub fn mark(&self, step: &'static str) {
        self.record(Call::Mark(step));
    }

    pub fn calls(&self) -> Vec<Call> {
        self.calls.borrow().clone()
    }

    pub fn clear(&self) {
        self.calls.borrow_mut().clear();
    }

    fn record(&self, call: Call) {
        self.calls.borrow_mut().push(call);
    }

    fn answer(&self, down: bool) -> Result<(), OutputError> {
        if self.panel.get() || (down && self.taken.get()) { Err(OutputError::Refused) } else { Ok(()) }
    }
}

impl ManagerPort for FakeManager {
    fn acquire_input(&self) -> Result<(), LeaseError> {
        self.record(Call::Acquire);
        match &self.holder {
            Some(holder) => Err(LeaseError::Refused { holder: holder.clone() }),
            None => Ok(()),
        }
    }

    fn release_input(&self) {
        self.record(Call::Release);
    }

    fn send_key(&self, vk: u32, down: bool) -> Result<(), OutputError> {
        self.record(Call::Key(vk, down));
        if down && let Some(error) = self.key_error.borrow().clone() {
            return Err(error);
        }
        self.answer(down)
    }

    fn send_move(&self, intent: MoveIntent) -> Result<(), OutputError> {
        self.record(Call::Move(intent));
        if !self.move_supported {
            return Err(OutputError::Unsupported);
        }
        if intent == MoveIntent::default() {
            return Ok(());
        }
        if let Some(error) = self.move_error.borrow().clone() {
            return Err(error);
        }
        self.answer(true)
    }

    fn send_pointer(&self, op: PointerOp) -> Result<(), OutputError> {
        self.record(Call::Pointer(op));
        let down = match op {
            PointerOp::Button { down, .. } => down,
            PointerOp::Wheel { .. } | PointerOp::Turn(_) => true,
        };
        self.answer(down)
    }

    fn select_hotbar(&self, index: u8) -> Result<(), OutputError> {
        self.record(Call::Hotbar(index));
        self.answer(true)
    }

    fn panel_open(&self) -> bool {
        self.panel.get()
    }
}
