use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use shiika_core::names::*;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Default)]
pub struct WitnessTable(HashMap<ModuleFullname, Vec<MethodFullname>>);

impl WitnessTable {
    pub fn build() -> Result<WitnessTable> {
        build_witness_table()
    }

    // Returns empty wtable.
    pub fn default() -> WitnessTable {
        WitnessTable(Default::default())
    }
}

fn build_witness_table() -> Result<WitnessTable> {
    Ok(Default::default())
}
