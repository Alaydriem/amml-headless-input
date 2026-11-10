use std::net::Ipv4Addr;

use hcl::{Body, Structure};

use super::raw::{RawDefaults, RawTop};
use super::{BodyAttributes, ControlFile, LineIndex, StepConverter, StepMeasure, Ticks};
use crate::errors::ControlError;
use crate::run::RunDefaults;
use crate::structs::Capabilities;

// Reads and checks a control file given as HCL text. Nothing runs until the whole file passes. A step that needs a
// player read the game build does not give is refused.
pub struct ControlLoader;

impl ControlLoader {
    // `resolve` gives the IPv4 address of a join_server host, or None when it has none
    pub fn load(
        text: &str,
        defaults: &RunDefaults,
        capabilities: &Capabilities,
        resolve: impl Fn(&str) -> Option<Ipv4Addr>,
    ) -> Result<ControlFile, ControlError> {
        if text.len() > ControlFile::MAX_BYTES {
            return Err(ControlError::new(None, format!("the file is over {} bytes", ControlFile::MAX_BYTES)));
        }
        let body = hcl::parse(text).map_err(|error| match &error {
            hcl::Error::Parse(parse) => ControlError::new(Some(parse.location().line()), parse.message().to_string()),
            other => ControlError::new(None, other.to_string()),
        })?;
        let index = LineIndex::new(text);
        let name = Self::name(&body, &index)?;
        let goal_timeout = Self::goal_timeout(&body, &index, defaults)?;
        let steps = StepConverter::new(&index, goal_timeout, *capabilities).steps(&body, 0, &resolve)?;
        let measure = StepMeasure::of(&steps);
        if measure.steps > ControlFile::MAX_STEPS {
            return Err(ControlError::new(
                None,
                format!("the file holds more than {} steps once repeats are expanded", ControlFile::MAX_STEPS),
            ));
        }
        if measure.ticks.is_some_and(|ticks| ticks > ControlFile::MAX_TICKS) {
            return Err(ControlError::new(None, format!("the file runs longer than {} ticks", ControlFile::MAX_TICKS)));
        }
        let total_ticks = measure.ticks.and_then(|ticks| u32::try_from(ticks).ok());
        Ok(ControlFile { name, steps, total_ticks })
    }

    fn name(body: &Body, index: &LineIndex) -> Result<String, ControlError> {
        if let Some(attribute) = body.attributes().find(|attribute| attribute.key() != "name") {
            return Err(ControlError::new(
                index.attribute_line(attribute.key()),
                format!("unknown attribute \"{}\"", attribute.key()),
            ));
        }
        let line = index.attribute_line("name");
        let raw: RawTop =
            hcl::from_body(BodyAttributes::of(body)).map_err(|error| ControlError::new(line, error.to_string()))?;
        Ok(raw.name.unwrap_or_else(|| "unnamed".to_string()))
    }

    fn goal_timeout(body: &Body, index: &LineIndex, defaults: &RunDefaults) -> Result<Ticks, ControlError> {
        let mut found = body.iter().filter_map(|structure| match structure {
            Structure::Block(block) if block.identifier() == "defaults" => Some(block),
            _ => None,
        });
        let Some(block) = found.next() else {
            return Ok(defaults.goal_timeout);
        };
        let mut cursor = 0;
        let line = index.block_line("defaults", None, &mut cursor);
        if found.next().is_some() {
            return Err(ControlError::new(line, "a file has one defaults block"));
        }
        let raw: RawDefaults = hcl::from_body(BodyAttributes::of(block.body()))
            .map_err(|error| ControlError::new(line, error.to_string()))?;
        match raw.goal_timeout {
            Some(text) => Ticks::parse(&text)
                .ok_or_else(|| ControlError::new(line, format!("goal_timeout \"{text}\" is not a duration"))),
            None => Ok(defaults.goal_timeout),
        }
    }
}
