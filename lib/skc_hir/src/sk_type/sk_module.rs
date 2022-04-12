use super::SkTypeBase;
use crate::signature::MethodSignature;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct SkModule {
    pub base: SkTypeBase,
    pub requirements: Vec<MethodSignature>,
}
