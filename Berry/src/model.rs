use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LightStatus {
    pub lane_id: u32,
    pub status: String, // e.g., "green", "yellow", "red"
}
