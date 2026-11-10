// Why the member registry did not take a registration.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegistryError {
    #[error("the hub holds the most members it takes ({0})")]
    Full(usize),
    #[error("instance {0} is used by another game")]
    Taken(String),
}
