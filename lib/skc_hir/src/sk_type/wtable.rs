use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use shiika_core::names::*;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Default)]
pub struct WTable(HashMap<ModuleFullname, Vec<MethodFullname>>);

impl WTable {
    pub fn build() -> Result<WTable> {
        build_wtable()
    }

    // Returns empty wtable.
    pub fn default() -> WTable {
        WTable(Default::default())
    }
}

fn build_wtable() -> Result<WTable> {
    Ok(Default::default())
}
