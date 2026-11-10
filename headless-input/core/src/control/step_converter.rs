use std::net::Ipv4Addr;
use std::sync::Arc;

use hcl::{Block, Body, Structure};
use serde::de::DeserializeOwned;

use super::raw::{RawHold, RawRepeat, RawStep};
use super::{BodyAttributes, ControlFile, DimensionName, InputKey, LineIndex, PointerAction, Step, Ticks};
use crate::errors::ControlError;
use crate::structs::Capabilities;

// Turns the blocks of a parsed control file into steps, naming the line of each refusal.
pub struct StepConverter<'a> {
    index: &'a LineIndex,
    goal_timeout: Ticks,
    capabilities: Capabilities,
    cursor: usize,
}

impl<'a> StepConverter<'a> {
    pub const DEFAULT_PORT: u16 = 19132;
    pub const DEFAULT_WITHIN: f64 = 0.5;
    // the smallest walk_to `within`: above one sprint tick (about 0.28 blocks), so a step cannot pass over the circle
    // and turn back without end
    pub const MIN_WITHIN: f64 = 0.3;
    // the most bytes of a command step text, with its slash
    pub const MAX_COMMAND_BYTES: usize = 256;

    pub fn new(index: &'a LineIndex, goal_timeout: Ticks, capabilities: Capabilities) -> Self {
        Self { index, goal_timeout, capabilities, cursor: 0 }
    }

    // the steps of a body at this repeat depth; its attributes belong to the caller
    // `resolve` gives the IPv4 address of a join_server host, or None when it has none
    pub fn steps(
        &mut self,
        body: &Body,
        depth: usize,
        resolve: &impl Fn(&str) -> Option<Ipv4Addr>,
    ) -> Result<Vec<Step>, ControlError> {
        let mut steps = Vec::new();
        for structure in body.iter() {
            let Structure::Block(block) = structure else {
                continue;
            };
            match block.identifier() {
                "step" => steps.push(self.step(block, resolve)?),
                "repeat" => steps.push(self.repeat(block, depth + 1, resolve)?),
                "defaults" if depth == 0 => {
                    self.index.block_line("defaults", None, &mut self.cursor);
                }
                other => {
                    let line = self.index.ident_line(other, &mut self.cursor);
                    return Err(ControlError::new(line, format!("unknown block \"{other}\"")));
                }
            }
        }
        Ok(steps)
    }

    fn read<T: DeserializeOwned>(body: &Body, line: Option<usize>) -> Result<T, ControlError> {
        hcl::from_body(BodyAttributes::of(body)).map_err(|error| ControlError::new(line, error.to_string()))
    }

    fn repeat(
        &mut self,
        block: &Block,
        depth: usize,
        resolve: &impl Fn(&str) -> Option<Ipv4Addr>,
    ) -> Result<Step, ControlError> {
        let line = self.index.block_line("repeat", None, &mut self.cursor);
        if depth > ControlFile::MAX_DEPTH {
            return Err(ControlError::new(line, format!("repeat blocks nest deeper than {}", ControlFile::MAX_DEPTH)));
        }
        let raw: RawRepeat = Self::read(block.body(), line)?;
        let count = match raw.count {
            None => None,
            Some(count) => match u32::try_from(count) {
                Ok(count) if count >= 1 => Some(count),
                _ => return Err(ControlError::new(line, "count must be a whole number of 1 or more")),
            },
        };
        let timeout = match &raw.timeout {
            Some(text) => Some(Self::duration(text, "timeout", line)?),
            None => None,
        };
        if count.is_none() && timeout.is_none() {
            return Err(ControlError::new(line, "a repeat with no count needs a timeout"));
        }
        let steps = self.steps(block.body(), depth, resolve)?;
        Ok(Step::Repeat { count, timeout, steps })
    }

    fn duration(text: &str, key: &str, line: Option<usize>) -> Result<Ticks, ControlError> {
        Ticks::parse(text).ok_or_else(|| {
            ControlError::new(
                line,
                format!("{key} \"{text}\" is not a duration (use \"5s\" or \"12t\", from 1 tick to one hour)"),
            )
        })
    }

    fn required(value: Option<&String>, key: &str, line: Option<usize>) -> Result<Ticks, ControlError> {
        match value {
            Some(text) => Self::duration(text, key, line),
            None => Err(ControlError::new(line, format!("{key} is required"))),
        }
    }

    fn need<T: Copy>(value: Option<T>, key: &str, line: Option<usize>) -> Result<T, ControlError> {
        value.ok_or_else(|| ControlError::new(line, format!("{key} is required")))
    }

    fn check_command(text: &str, line: Option<usize>) -> Result<(), ControlError> {
        if !text.starts_with('/') {
            return Err(ControlError::new(line, "text must start with \"/\""));
        }
        if text.len() > Self::MAX_COMMAND_BYTES {
            let why = format!("text must be {} bytes or fewer", Self::MAX_COMMAND_BYTES);
            return Err(ControlError::new(line, why));
        }
        if text.contains(['\n', '\r', '\0']) {
            return Err(ControlError::new(line, "text must not hold a newline or a NUL"));
        }
        Ok(())
    }

    fn goal_timeout(&self, raw: &RawStep, line: Option<usize>) -> Result<Ticks, ControlError> {
        match &raw.timeout {
            Some(text) => Self::duration(text, "timeout", line),
            None => Ok(self.goal_timeout),
        }
    }

    fn allowed(kind: &str) -> Option<&'static [&'static str]> {
        Some(match kind {
            "move" => &["hold", "for"],
            "jump" | "sneak" | "sprint" | "wait" => &["for"],
            "turn" | "look_at" => &["yaw", "pitch", "over"],
            "attack" | "use" => &["hold"],
            "hotbar" => &["slot"],
            "scroll" => &["steps"],
            "command" => &["text"],
            "walk_to" => &["x", "y", "z", "within", "sprint", "timeout"],
            "walk_until_dimension" => &["dimension", "timeout"],
            "join_server" => &["address", "port", "timeout"],
            "leave_world" => &["timeout"],
            _ => return None,
        })
    }

    fn step(&mut self, block: &Block, resolve: &impl Fn(&str) -> Option<Ipv4Addr>) -> Result<Step, ControlError> {
        let kind = match block.labels() {
            [label] => label.as_str().to_string(),
            _ => {
                let line = self.index.ident_line("step", &mut self.cursor);
                return Err(ControlError::new(line, "a step has exactly one label, its kind"));
            }
        };
        let line = self.index.block_line("step", Some(&kind), &mut self.cursor);
        let Some(allowed) = Self::allowed(&kind) else {
            return Err(ControlError::new(line, format!("unknown step kind \"{kind}\"")));
        };
        if block.body().blocks().next().is_some() {
            return Err(ControlError::new(line, "a step holds no blocks"));
        }
        for attribute in block.body().attributes() {
            if !allowed.contains(&attribute.key()) {
                return Err(ControlError::new(
                    line,
                    format!("step \"{kind}\" has no attribute \"{}\"", attribute.key()),
                ));
            }
        }
        let raw: RawStep = Self::read(block.body(), line)?;
        let step = self.build(&kind, &raw, line, resolve)?;
        match self.capabilities.missing_for(&step) {
            Some(read) => Err(ControlError::new(
                line,
                format!("step \"{kind}\" needs {read}, which this game build does not give"),
            )),
            None => Ok(step),
        }
    }

    fn build(
        &self,
        kind: &str,
        raw: &RawStep,
        line: Option<usize>,
        resolve: &impl Fn(&str) -> Option<Ipv4Addr>,
    ) -> Result<Step, ControlError> {
        Ok(match kind {
            "move" => {
                let names = match &raw.hold {
                    Some(RawHold::Many(names)) => names.clone(),
                    Some(RawHold::One(name)) => vec![name.clone()],
                    None => return Err(ControlError::new(line, "hold is required")),
                };
                if names.is_empty() {
                    return Err(ControlError::new(line, "hold names no key"));
                }
                let mut hold = Vec::with_capacity(names.len());
                for name in &names {
                    hold.push(
                        InputKey::from_name(name)
                            .ok_or_else(|| ControlError::new(line, format!("unknown key \"{name}\"")))?,
                    );
                }
                Step::Move { hold, ticks: Self::required(raw.for_time.as_ref(), "for", line)? }
            }
            "jump" => Step::Jump {
                ticks: match &raw.for_time {
                    Some(text) => Self::duration(text, "for", line)?,
                    None => Ticks(1),
                },
            },
            "sneak" => Step::Sneak { ticks: Self::required(raw.for_time.as_ref(), "for", line)? },
            "sprint" => Step::Sprint { ticks: Self::required(raw.for_time.as_ref(), "for", line)? },
            "wait" => Step::Wait { ticks: Self::required(raw.for_time.as_ref(), "for", line)? },
            "turn" => Step::Turn {
                yaw: raw.yaw.unwrap_or(0.0) as f32,
                pitch: raw.pitch.unwrap_or(0.0) as f32,
                ticks: Self::required(raw.over.as_ref(), "over", line)?,
            },
            "look_at" => Step::LookAt {
                yaw: Self::need(raw.yaw, "yaw", line)? as f32,
                pitch: Self::need(raw.pitch, "pitch", line)? as f32,
                ticks: Self::required(raw.over.as_ref(), "over", line)?,
            },
            "attack" | "use" => {
                let action = if kind == "attack" { PointerAction::Attack } else { PointerAction::Use };
                let hold_ticks = match &raw.hold {
                    None => 0,
                    Some(RawHold::One(text)) => Self::duration(text, "hold", line)?.get(),
                    Some(RawHold::Many(_)) => {
                        return Err(ControlError::new(line, "hold is a duration for this step"));
                    }
                };
                Step::Pointer { action, hold_ticks }
            }
            "hotbar" => {
                let slot = Self::need(raw.slot, "slot", line)?;
                match u8::try_from(slot) {
                    Ok(slot @ 1..=9) => Step::Hotbar { slot },
                    _ => return Err(ControlError::new(line, "slot must be from 1 to 9")),
                }
            }
            "scroll" => {
                let steps = Self::need(raw.steps, "steps", line)?;
                match i32::try_from(steps) {
                    Ok(steps) if steps != 0 => Step::Scroll { steps },
                    _ => return Err(ControlError::new(line, "steps must be a whole number that is not 0")),
                }
            }
            "command" => {
                let Some(text) = &raw.text else {
                    return Err(ControlError::new(line, "text is required"));
                };
                Self::check_command(text, line)?;
                Step::Command { text: Arc::from(text.as_str()) }
            }
            "walk_to" => {
                let within = raw.within.unwrap_or(Self::DEFAULT_WITHIN);
                if within.is_nan() || within < Self::MIN_WITHIN {
                    let why = format!("within must be {} blocks or more", Self::MIN_WITHIN);
                    return Err(ControlError::new(line, why));
                }
                Step::WalkTo {
                    x: Self::need(raw.x, "x", line)? as f32,
                    z: Self::need(raw.z, "z", line)? as f32,
                    y: raw.y.map(|y| y as f32),
                    within: within as f32,
                    sprint: raw.sprint.unwrap_or(false),
                    timeout: self.goal_timeout(raw, line)?,
                }
            }
            "walk_until_dimension" => {
                let dimension = match &raw.dimension {
                    Some(name) => Some(
                        DimensionName::from_name(name)
                            .ok_or_else(|| ControlError::new(line, format!("unknown dimension \"{name}\"")))?,
                    ),
                    None => None,
                };
                Step::WalkUntilDimension { dimension, timeout: self.goal_timeout(raw, line)? }
            }
            "join_server" => {
                let host = match &raw.address {
                    Some(address) if !address.trim().is_empty() => address.clone(),
                    _ => return Err(ControlError::new(line, "address is required")),
                };
                let Some(address) = resolve(&host) else {
                    return Err(ControlError::new(
                        line,
                        format!("address \"{host}\" did not resolve to an IPv4 address"),
                    ));
                };
                let port = match raw.port {
                    None => Self::DEFAULT_PORT,
                    Some(port) => match u16::try_from(port) {
                        Ok(port) if port >= 1 => port,
                        _ => return Err(ControlError::new(line, "port must be from 1 to 65535")),
                    },
                };
                Step::JoinServer { host, address, port, timeout: self.goal_timeout(raw, line)? }
            }
            _ => Step::LeaveWorld { timeout: self.goal_timeout(raw, line)? },
        })
    }
}
