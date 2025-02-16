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
    // For OutputBoundary lanes, start_intersection is the grid intersection and end_intersection is 0.
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
    // For InputBoundary lanes, start_intersection is 0 and end_intersection is the grid intersection.
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
    // The destination is now hard-coded based on the original direction.
    // Row 2 (Intersections 5..8) horizontally:
    lanes.push(Lane { // from 5 east to 6
        id: lane_id,
        start_intersection: 1,
        end_intersection: 2,
        length: 300.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { // from 6 west to 5
        id: lane_id,
        start_intersection: 2,
        end_intersection: 3,
        length: 500.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { // from 6 east to 7
        id: lane_id,
        start_intersection: 3,
        end_intersection: 4,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { // from 7 west to 6
        id: lane_id,
        start_intersection: 4,
        end_intersection: 8,
        length: 300.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { // from 7 east to 8
        id: lane_id,
        start_intersection: 5,
        end_intersection: 1,
        length: 300.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { // from 8 west to 7
        id: lane_id,
        start_intersection: 5,
        end_intersection: 6,
        length: 500.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;

    // Column 1 between 5 and 9:
    lanes.push(Lane { // from 5 south to 9
        id: lane_id,
        start_intersection: 5,
        end_intersection: 9,
        length: 400.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { // from 9 north to 5
        id: lane_id,
        start_intersection: 6,
        end_intersection: 5,
        length: 500.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;

    // Row 3 (Intersections 9..12) horizontally:
    lanes.push(Lane { // from 9 east to 10
        id: lane_id,
        start_intersection: 2,
        end_intersection: 6,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { // from 10 west to 9
        id: lane_id,
        start_intersection: 6,
        end_intersection: 2,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { // from 10 east to 11
        id: lane_id,
        start_intersection: 6,
        end_intersection: 7,
        length: 300.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { // from 11 west to 10
        id: lane_id,
        start_intersection: 7,
        end_intersection: 6,
        length: 300.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { // from 11 east to 12
        id: lane_id,
        start_intersection: 7,
        end_intersection: 3,
        length: 300.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { // from 12 west to 11
        id: lane_id,
        start_intersection: 7,
        end_intersection: 8,
        length: 300.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;

    // Column 2 between 6 & 10, and 10 & 14:
    lanes.push(Lane { // from 6 south to 10
        id: lane_id,
        start_intersection: 8,
        end_intersection: 7,
        length: 300.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { // from 10 north to 6
        id: lane_id,
        start_intersection: 8,
        end_intersection: 12,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { // from 10 south to 14
        id: lane_id,
        start_intersection: 9,
        end_intersection: 10,
        length: 100.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { // from 14 north to 10
        id: lane_id,
        start_intersection: 9,
        end_intersection: 13,
        length: 400.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;

    // Column 3 between 7 & 11:
    lanes.push(Lane { // from 7 south to 11
        id: lane_id,
        start_intersection: 10,
        end_intersection: 9,
        length: 100.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { // from 11 north to 7
        id: lane_id,
        start_intersection: 10,
        end_intersection: 11,
        length: 150.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;

    // Column 4 between 8 & 12:
    lanes.push(Lane { // from 8 south to 12
        id: lane_id,
        start_intersection: 10,
        end_intersection: 14,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { // from 12 north to 8
        id: lane_id,
        start_intersection: 11,
        end_intersection: 10,
        length: 150.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;

    // Column 1 between 9 & 13:
    lanes.push(Lane { // from 9 south to 13
        id: lane_id,
        start_intersection: 11,
        end_intersection: 7,
        length: 500.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { // from 13 north to 9
        id: lane_id,
        start_intersection: 11,
        end_intersection: 15,
        length: 400.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;

    // Column 3 between 11 & 15:
    lanes.push(Lane { // from 11 south to 15
        id: lane_id,
        start_intersection: 12,
        end_intersection: 8,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { // from 15 north to 11
        id: lane_id,
        start_intersection: 12,
        end_intersection: 16,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;

    // Column 4 between 12 & 16:
    lanes.push(Lane { // from 12 south to 16
        id: lane_id,
        start_intersection: 14,
        end_intersection: 13,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { // from 16 north to 12
        id: lane_id,
        start_intersection: 14,
        end_intersection: 10,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;

    // Extra horizontal lanes in row 2/3:
    lanes.push(Lane { // from 6 east to 7
        id: lane_id,
        start_intersection: 14,
        end_intersection: 15,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { // from 7 west to 6
        id: lane_id,
        start_intersection: 15,
        end_intersection: 14,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { // from 10 east to 11
        id: lane_id,
        start_intersection: 15,
        end_intersection: 11,
        length: 400.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { // from 11 west to 10
        id: lane_id,
        start_intersection: 15,
        end_intersection: 16,
        length: 500.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { // from 14 east to 15
        id: lane_id,
        start_intersection: 16,
        end_intersection: 12,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;
    lanes.push(Lane { // from 15 west to 14
        id: lane_id,
        start_intersection: 16,
        end_intersection: 15,
        length: 500.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1;

    lanes
}
