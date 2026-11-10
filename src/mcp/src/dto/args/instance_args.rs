use schemars::JsonSchema;
use serde::Deserialize;

// The arguments of `stop`, `status` and `release`: only the instance.
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
pub struct InstanceArgs {
    /// The instance id; may be left out when one instance is connected. `"*"` is every instance (stop and release).
    pub instance: Option<String>,
}
