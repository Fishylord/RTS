use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LightStatus {
    pub lane_id: u32,
    pub status: String, // e.g., "green", "yellow", "red"
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimulationConfig {
    pub base_timing: f64,
    pub acceleration: f64, // New field for simulation relative speed
}

impl Default for SimulationConfig {
    fn default() -> Self {
        SimulationConfig { 
            base_timing: 35.0,
            acceleration: 250.0, // Default recommended acceleration
        }
    }
}

pub fn load_simulation_config() -> SimulationConfig {
    use std::fs;
    let path = "sim_config.json";
    if let Ok(contents) = fs::read_to_string(path) {
        if let Ok(config) = serde_json::from_str::<SimulationConfig>(&contents) {
            return config;
        }
    }
    SimulationConfig::default()
}
