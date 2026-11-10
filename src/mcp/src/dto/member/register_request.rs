use serde::{Deserialize, Serialize};

use crate::dto::FeaturesDto;

// The body of `POST /member/register`: who the member is and what its game build gives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub id: String,
    pub pid: u32,
    pub features: FeaturesDto,
}
