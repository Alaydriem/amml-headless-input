use schemars::JsonSchema;
use serde::Deserialize;

// The arguments of `run_control_file`: a path to a control file, or its text.
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
pub struct RunControlFileArgs {
    /// The instance id; may be left out when one instance is connected. `"*"` runs on every instance.
    pub instance: Option<String>,
    /// The path of an HCL control file, read by the hub. A path that is not absolute, such as a bare file name, is
    /// read from the control folder of the hub instance (the files.folder setting).
    pub path: Option<String>,
    /// The HCL text of a control file.
    pub text: Option<String>,
}
