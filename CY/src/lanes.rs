// lanes.rs
//
// This file provides a LaneCategory enum, a Lane struct, and a function
// load_lanes() that returns a Vec of 52 lanes (18 boundary + 34 internal).
// Each lane is tagged as InputBoundary, OutputBoundary, or Internal.

use crate::simulation::Direction;

/// Distinguishes whether a lane is an input boundary (entry), output boundary (exit), or internal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneCategory {
    InputBoundary,
    OutputBoundary,
    Internal,
}

/// A lane in the system.
#[derive(Debug, Clone)]
pub struct Lane {
    pub id: u32,
    pub intersection: u32,
    pub direction: Direction,
    pub length: f64,
    pub category: LaneCategory,
}

pub fn load_lanes() -> Vec<Lane> {
    let mut lanes = Vec::new();
    let mut lane_id = 1000;

    //
    // ----------------------------------------------------------------
    // 1) OUTPUT BOUNDARY LANES (10 total)
    // ----------------------------------------------------------------
    // Example arrangement based on prior table interpretation:
    //  (Feel free to adjust if your exact scenario differs.)
    //
    lanes.push(Lane {
        id: lane_id,
        intersection: 1,
        direction: Direction::North,
        length: 100.0,
        category: LaneCategory::OutputBoundary,
    });
    lane_id += 1; // #1000

    lanes.push(Lane {
        id: lane_id,
        intersection: 2,
        direction: Direction::North,
        length: 300.0,
        category: LaneCategory::OutputBoundary,
    });
    lane_id += 1; // #1001

    lanes.push(Lane {
        id: lane_id,
        intersection: 3,
        direction: Direction::North,
        length: 300.0,
        category: LaneCategory::OutputBoundary,
    });
    lane_id += 1; // #1002

    lanes.push(Lane {
        id: lane_id,
        intersection: 4,
        direction: Direction::East,
        length: 200.0,
        category: LaneCategory::OutputBoundary,
    });
    lane_id += 1; // #1003

    lanes.push(Lane {
        id: lane_id,
        intersection: 5,
        direction: Direction::West,
        length: 400.0,
        category: LaneCategory::OutputBoundary,
    });
    lane_id += 1; // #1004

    lanes.push(Lane {
        id: lane_id,
        intersection: 12,
        direction: Direction::East,
        length: 400.0,
        category: LaneCategory::OutputBoundary,
    });
    lane_id += 1; // #1005

    lanes.push(Lane {
        id: lane_id,
        intersection: 13,
        direction: Direction::West,
        length: 200.0,
        category: LaneCategory::OutputBoundary,
    });
    lane_id += 1; // #1006

    lanes.push(Lane {
        id: lane_id,
        intersection: 13,
        direction: Direction::South,
        length: 200.0,
        category: LaneCategory::OutputBoundary,
    });
    lane_id += 1; // #1007

    lanes.push(Lane {
        id: lane_id,
        intersection: 15,
        direction: Direction::South,
        length: 200.0,
        category: LaneCategory::OutputBoundary,
    });
    lane_id += 1; // #1008

    lanes.push(Lane {
        id: lane_id,
        intersection: 16,
        direction: Direction::South,
        length: 400.0,
        category: LaneCategory::OutputBoundary,
    });
    lane_id += 1; // #1009

    //
    // ----------------------------------------------------------------
    // 2) INPUT BOUNDARY LANES (8 total)
    // ----------------------------------------------------------------
    //
    lanes.push(Lane {
        id: lane_id,
        intersection: 1,
        direction: Direction::East,
        length: 200.0,
        category: LaneCategory::InputBoundary,
    });
    lane_id += 1; // #1010

    lanes.push(Lane {
        id: lane_id,
        intersection: 2,
        direction: Direction::South,
        length: 300.0,
        category: LaneCategory::InputBoundary,
    });
    lane_id += 1; // #1011

    lanes.push(Lane {
        id: lane_id,
        intersection: 4,
        direction: Direction::South,
        length: 100.0,
        category: LaneCategory::InputBoundary,
    });
    lane_id += 1; // #1012

    lanes.push(Lane {
        id: lane_id,
        intersection: 5,
        direction: Direction::East,
        length: 400.0,
        category: LaneCategory::InputBoundary,
    });
    lane_id += 1; // #1013

    lanes.push(Lane {
        id: lane_id,
        intersection: 12,
        direction: Direction::West,
        length: 400.0,
        category: LaneCategory::InputBoundary,
    });
    lane_id += 1; // #1014

    lanes.push(Lane {
        id: lane_id,
        intersection: 15,
        direction: Direction::North,
        length: 200.0,
        category: LaneCategory::InputBoundary,
    });
    lane_id += 1; // #1015

    lanes.push(Lane {
        id: lane_id,
        intersection: 16,
        direction: Direction::West,
        length: 500.0,
        category: LaneCategory::InputBoundary,
    });
    lane_id += 1; // #1016

    lanes.push(Lane {
        id: lane_id,
        intersection: 16,
        direction: Direction::North,
        length: 400.0,
        category: LaneCategory::InputBoundary,
    });
    lane_id += 1; // #1017

    //
    // ----------------------------------------------------------------
    // 3) INTERNAL LANES (34 total)
    // ----------------------------------------------------------------
    //
    // Below is an example grouping; adapt as you see fit.
    // Row 2 (Intersections 5..8), 6 lanes horizontally
    lanes.push(Lane {
        id: lane_id,
        intersection: 5,
        direction: Direction::East,
        length: 400.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1018
    lanes.push(Lane {
        id: lane_id,
        intersection: 6,
        direction: Direction::West,
        length: 400.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1019
    lanes.push(Lane {
        id: lane_id,
        intersection: 6,
        direction: Direction::East,
        length: 500.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1020
    lanes.push(Lane {
        id: lane_id,
        intersection: 7,
        direction: Direction::West,
        length: 500.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1021
    lanes.push(Lane {
        id: lane_id,
        intersection: 7,
        direction: Direction::East,
        length: 300.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1022
    lanes.push(Lane {
        id: lane_id,
        intersection: 8,
        direction: Direction::West,
        length: 300.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1023

    // Column 1 between 5 and 9 => 2 lanes
    lanes.push(Lane {
        id: lane_id,
        intersection: 5,
        direction: Direction::South,
        length: 400.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1024
    lanes.push(Lane {
        id: lane_id,
        intersection: 9,
        direction: Direction::North,
        length: 400.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1025

    // Row 3 (Intersections 9..12), 6 lanes horizontally
    lanes.push(Lane {
        id: lane_id,
        intersection: 9,
        direction: Direction::East,
        length: 100.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1026
    lanes.push(Lane {
        id: lane_id,
        intersection: 10,
        direction: Direction::West,
        length: 100.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1027
    lanes.push(Lane {
        id: lane_id,
        intersection: 10,
        direction: Direction::East,
        length: 150.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1028
    lanes.push(Lane {
        id: lane_id,
        intersection: 11,
        direction: Direction::West,
        length: 150.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1029
    lanes.push(Lane {
        id: lane_id,
        intersection: 11,
        direction: Direction::East,
        length: 400.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1030
    lanes.push(Lane {
        id: lane_id,
        intersection: 12,
        direction: Direction::West,
        length: 400.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1031

    // Column 2 between 6 & 10, and 10 & 14 => 4 lanes
    lanes.push(Lane {
        id: lane_id,
        intersection: 6,
        direction: Direction::South,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1032
    lanes.push(Lane {
        id: lane_id,
        intersection: 10,
        direction: Direction::North,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1033
    lanes.push(Lane {
        id: lane_id,
        intersection: 10,
        direction: Direction::South,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1034
    lanes.push(Lane {
        id: lane_id,
        intersection: 14,
        direction: Direction::North,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1035

    // Column 3 between 7 & 11 => 2 lanes
    lanes.push(Lane {
        id: lane_id,
        intersection: 7,
        direction: Direction::South,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1036
    lanes.push(Lane {
        id: lane_id,
        intersection: 11,
        direction: Direction::North,
        length: 200.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1037

    // Column 4 between 8 & 12 => 2 lanes
    lanes.push(Lane {
        id: lane_id,
        intersection: 8,
        direction: Direction::South,
        length: 300.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1038
    lanes.push(Lane {
        id: lane_id,
        intersection: 12,
        direction: Direction::North,
        length: 300.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1039

    // Column 1 between 9 & 13 => 2 lanes
    lanes.push(Lane {
        id: lane_id,
        intersection: 9,
        direction: Direction::South,
        length: 400.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1040
    lanes.push(Lane {
        id: lane_id,
        intersection: 13,
        direction: Direction::North,
        length: 400.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1041

    // Column 3 between 11 & 15 => 2 lanes
    lanes.push(Lane {
        id: lane_id,
        intersection: 11,
        direction: Direction::South,
        length: 400.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1042
    lanes.push(Lane {
        id: lane_id,
        intersection: 15,
        direction: Direction::North,
        length: 400.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1043

    // Column 4 between 12 & 16 => 2 lanes
    lanes.push(Lane {
        id: lane_id,
        intersection: 12,
        direction: Direction::South,
        length: 400.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1044
    lanes.push(Lane {
        id: lane_id,
        intersection: 16,
        direction: Direction::North,
        length: 400.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1045

    // A few extra horizontal lanes in row 2/3 (6 more => total 34 internal)
    lanes.push(Lane {
        id: lane_id,
        intersection: 6,
        direction: Direction::East,
        length: 300.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1046
    lanes.push(Lane {
        id: lane_id,
        intersection: 7,
        direction: Direction::West,
        length: 300.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1047
    lanes.push(Lane {
        id: lane_id,
        intersection: 10,
        direction: Direction::East,
        length: 300.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1048
    lanes.push(Lane {
        id: lane_id,
        intersection: 11,
        direction: Direction::West,
        length: 300.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1049
    lanes.push(Lane {
        id: lane_id,
        intersection: 14,
        direction: Direction::East,
        length: 300.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1050
    lanes.push(Lane {
        id: lane_id,
        intersection: 15,
        direction: Direction::West,
        length: 300.0,
        category: LaneCategory::Internal,
    });
    lane_id += 1; // #1051

    // Now we have:
    //   10 output + 8 input = 18 boundary lanes
    //   + 34 internal lanes
    //   = 52 total lanes.
    lanes
}
