use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

const STATE_PATH: &str = "/var/lib/hpm/state.json";

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct State {
    pub packages: HashMap<String, HashMap<String, String>>,
}

pub fn load_state() -> Result<State> {
    if !Path::new(STATE_PATH).exists() {
        return Ok(State::default());
    }
    let data = fs::read(STATE_PATH)?;
    serde_json::from_slice(&data).map_err(Into::into)
}

pub fn save_state(state: &State) -> Result<()> {
    let data = serde_json::to_vec(state)?;
    let tmp_path = format!("{}.tmp", STATE_PATH);
    fs::write(&tmp_path, data)?;
    fs::rename(&tmp_path, STATE_PATH)?;
    Ok(())
}

pub fn update_state(package_name: &str, version: &str, checksum: &str) -> Result<()> {
    let mut state = load_state()?;
    state
    .packages
    .entry(package_name.to_string())
    .or_insert_with(HashMap::new)
    .insert(version.to_string(), checksum.to_string());
    save_state(&state)
}
