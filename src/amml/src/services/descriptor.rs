// the descriptor the manager registers the mod from; it names the mod, the claims and the settings schema
pub struct HeadlessDescriptor;

impl HeadlessDescriptor {
    pub const TEXT: &'static str = include_str!("../../descriptor.hcl");
}
