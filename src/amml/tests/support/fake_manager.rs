use std::cell::{Cell, RefCell};

use alaydriem_headless_input::errors::{LeaseError, OutputError};
use alaydriem_headless_input::traits::ManagerPort;
use amm_core::structs::input::PointerOp;

// one call the output side made on the manager
#[derive(Debug, Clone, PartialEq)]
pub enum Call {
    Acquire,
    Release,
    Key(u32, bool),
    Pointer(PointerOp),
    // a step outside the manager the test wrote into the record, to check the order
    Mark(&'static str),
}

// a manager stand-in: every call is recorded in order. A lease is granted unless a holder is set. While the panel
// is open every send is refused; while another mod holds the input every down is refused and ups are taken
pub struct FakeManager {
    calls: RefCell<Vec<Call>>,
    holder: Option<String>,
    panel: Cell<bool>,
    taken: Cell<bool>,
}

impl FakeManager {
    pub fn new() -> Self {
        Self { calls: RefCell::new(Vec::new()), holder: None, panel: Cell::new(false), taken: Cell::new(false) }
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
        self.answer(down)
    }

    fn send_pointer(&self, op: PointerOp) -> Result<(), OutputError> {
        self.record(Call::Pointer(op));
        let down = match op {
            PointerOp::Button { down, .. } => down,
            PointerOp::Wheel { .. } | PointerOp::Turn { .. } => true,
        };
        self.answer(down)
    }

    fn panel_open(&self) -> bool {
        self.panel.get()
    }
}
