#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneCategory {
    InputBoundary,
    OutputBoundary,
    Internal,
}

#[derive(Debug, Clone)]
pub struct Lane {
    pub id: u32,
    pub start_intersection: u32,
    pub end_intersection: u32,
    pub length: f64,
    pub category: LaneCategory,
}

pub fn load_lanes() -> Vec<Lane> {
    let mut lanes = Vec::new();
    let mut lane_id = 1000;

    // 1) OUTPUT BOUNDARY LANES (10 total)
    lanes.push(Lane {
        id: lane_id,
        start_intersection: 1,
        end_intersection: 0,
        length: 100.0,
        category: LaneCategory::OutputBoundary,
    });
    lane_id += 1;
    lanes.push(Lane {
        id: lane_id,
        start_intersection: 2,
        end_intersection: 0,
        length: 300.0,
        category: LaneCategory::OutputBoundary,
    });
    lane_id += 1;
    lanes.push(Lane {
        id: lane_id,
        start_intersection: 3,
        end_intersection: 0,
        length: 300.0,
        category: LaneCategory::OutputBoundary,
    });
    lane_id += 1;
    lanes.push(Lane {
        id: lane_id,
        start_intersection: 4,
        end_intersection: 0,
        length: 200.0,
        category: LaneCategory::OutputBoundary,
    });
    lane_id += 1;
    lanes.push(Lane {
        id: lane_id,
        start_intersection: 5,
        end_intersection: 0,
        length: 400.0,
        category: LaneCategory::OutputBoundary,
    });
    lane_id += 1;
    lanes.push(Lane {
        id: lane_id,
        start_intersection: 12,
        end_intersection: 0,
        length: 400.0,
        category: LaneCategory::OutputBoundary,
    });
    lane_id += 1;
    lanes.push(Lane {
        id: lane_id,
        start_intersection: 13,
        end_intersection: 0,
        length: 200.0,
        category: LaneCategory::OutputBoundary,
    });
    lane_id += 1;
    lanes.push(Lane {
        id: lane_id,
        start_intersection: 13,
        end_intersection: 0,
        length: 200.0,
        category: LaneCategory::OutputBoundary,
    });
    lane_id += 1;
    lanes.push(Lane {
        id: lane_id,
        start_intersection: 15,
        end_intersection: 0,
        length: 200.0,
        category: LaneCategory::OutputBoundary,
    });
    lane_id += 1;
    lanes.push(Lane {
        id: lane_id,
        start_intersection: 16,
        end_intersection: 0,
        length: 400.0,
        category: LaneCategory::OutputBoundary,
    });
    lane_id += 1;

    // 2) INPUT BOUNDARY LANES (8 total)
    lanes.push(Lane {
        id: lane_id,
        start_intersection: 0,
        end_intersection: 1,
        length: 200.0,
        category: LaneCategory::InputBoundary,
    });
    lane_id += 1;
    lanes.push(Lane {
        id: lane_id,
        start_intersection: 0,
        end_intersection: 2,
        length: 300.0,
        category: LaneCategory::InputBoundary,
    });
    lane_id += 1;
    lanes.push(Lane {
        id: lane_id,
        start_intersection: 0,
        end_intersection: 4,
        length: 100.0,
        category: LaneCategory::InputBoundary,
    });
    lane_id += 1;
    lanes.push(Lane {
        id: lane_id,
        start_intersection: 0,
        end_intersection: 5,
        length: 400.0,
        category: LaneCategory::InputBoundary,
    });
    lane_id += 1;
    lanes.push(Lane {
        id: lane_id,
        start_intersection: 0,
        end_intersection: 12,
        length: 400.0,
        category: LaneCategory::InputBoundary,
    });
    lane_id += 1;
    lanes.push(Lane {
        id: lane_id,
        start_intersection: 0,
        end_intersection: 15,
        length: 200.0,
        category: LaneCategory::InputBoundary,
    });
    lane_id += 1;
    lanes.push(Lane {
        id: lane_id,
        start_intersection: 0,
        end_intersection: 16,
        length: 500.0,
        category: LaneCategory::InputBoundary,
    });
    lane_id += 1;
    lanes.push(Lane {
        id: lane_id,
        start_intersection: 0,
        end_intersection: 16,
        length: 400.0,
        category: LaneCategory::InputBoundary,
    });
    lane_id += 1;

    // 3) INTERNAL LANES (34 total)
    // (Internal lane definitions omitted for brevity.)
    lanes
}
