// lanes.rs
//
// This file provides a LaneCategory enum, a Lane struct, and a function
// load_lanes() that returns a Vec of 52 lanes (18 boundary + 34 internal).
// Each lane is tagged as InputBoundary, OutputBoundary, or Internal.
//
// The Direction field has been removed. Instead, each lane now has two fields:
//   - start_intersection: for boundary lanes, this is the junction on the grid (for output lanes)
//     or 0 (for input lanes coming from outside).
//   - end_intersection: for boundary lanes, this is the junction on the grid (for input lanes)
//     or 0 (for output lanes exiting the grid).
// For internal lanes, both start and end intersections are specified based on the previous direction.

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
    lanes.push(Lane { 
        id: lane_id,
        start_intersection: 1,
        end_intersection: 2,
        length: 300.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { 
        id: lane_id,
        start_intersection: 2,
        end_intersection: 3,
        length: 500.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane {
        id: lane_id,
        start_intersection: 3,
        end_intersection: 4,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { 
        id: lane_id,
        start_intersection: 4,
        end_intersection: 8,
        length: 300.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { 
        id: lane_id,
        start_intersection: 5,
        end_intersection: 1,
        length: 300.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane {
        id: lane_id,
        start_intersection: 5,
        end_intersection: 6,
        length: 500.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;

    
    lanes.push(Lane {
        id: lane_id,
        start_intersection: 5,
        end_intersection: 9,
        length: 400.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { 
        id: lane_id,
        start_intersection: 6,
        end_intersection: 5,
        length: 500.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;

    
    lanes.push(Lane { 
        id: lane_id,
        start_intersection: 2,
        end_intersection: 6,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { 
        id: lane_id,
        start_intersection: 6,
        end_intersection: 2,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { 
        id: lane_id,
        start_intersection: 6,
        end_intersection: 7,
        length: 300.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { 
        id: lane_id,
        start_intersection: 7,
        end_intersection: 6,
        length: 300.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { 
        id: lane_id,
        start_intersection: 7,
        end_intersection: 3,
        length: 300.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { 
        id: lane_id,
        start_intersection: 7,
        end_intersection: 8,
        length: 300.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;

    
    lanes.push(Lane { 
        id: lane_id,
        start_intersection: 8,
        end_intersection: 7,
        length: 300.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { 
        id: lane_id,
        start_intersection: 8,
        end_intersection: 12,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane {
        id: lane_id,
        start_intersection: 9,
        end_intersection: 10,
        length: 100.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { 
        id: lane_id,
        start_intersection: 9,
        end_intersection: 13,
        length: 400.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;

    
    lanes.push(Lane { 
        id: lane_id,
        start_intersection: 10,
        end_intersection: 9,
        length: 100.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { 
        id: lane_id,
        start_intersection: 10,
        end_intersection: 11,
        length: 150.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;

    
    lanes.push(Lane {
        id: lane_id,
        start_intersection: 10,
        end_intersection: 14,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { 
        id: lane_id,
        start_intersection: 11,
        end_intersection: 10,
        length: 150.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;

    
    lanes.push(Lane { 
        id: lane_id,
        start_intersection: 11,
        end_intersection: 7,
        length: 500.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { 
        id: lane_id,
        start_intersection: 11,
        end_intersection: 15,
        length: 400.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;

    
    lanes.push(Lane {
        id: lane_id,
        start_intersection: 12,
        end_intersection: 8,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { 
        id: lane_id,
        start_intersection: 12,
        end_intersection: 16,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;

    
    lanes.push(Lane { 
        id: lane_id,
        start_intersection: 14,
        end_intersection: 13,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { 
        id: lane_id,
        start_intersection: 14,
        end_intersection: 10,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;

    
    lanes.push(Lane { 
        id: lane_id,
        start_intersection: 14,
        end_intersection: 15,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane {
        id: lane_id,
        start_intersection: 15,
        end_intersection: 14,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { 
        id: lane_id,
        start_intersection: 15,
        end_intersection: 11,
        length: 400.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane {
        id: lane_id,
        start_intersection: 15,
        end_intersection: 16,
        length: 500.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { 
        id: lane_id,
        start_intersection: 16,
        end_intersection: 12,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { 
        id: lane_id,
        start_intersection: 16,
        end_intersection: 15,
        length: 500.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;

    lanes
}
