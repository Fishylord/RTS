use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

// Import the LogEvent type from the monitoring module.
use crate::system_monitoring::LogEvent;

/// Simulation events that the Simulation Engine produces.
/// These events are sent to the Flow Analyzer.
#[derive(Debug)]
pub enum SimEvent {
    VehicleArrival {
        vehicle_id: u32,
        junction: u32, // Junction IDs: 1 to 4.
        lane: u32,     // Lane ID: each junction has 3 lanes (total 12 lanes).
        timestamp: u64, // Simulated time in seconds.
    },
    VehicleDeparture {
        vehicle_id: u32,
        junction: u32,
        lane: u32,
        timestamp: u64,
    },
    TrafficUpdate {
        junction: u32,
        vehicle_count: u32,
        timestamp: u64,
    },
}

/// Runs the traffic simulation for two days of simulated time.
/// The simulation is accelerated so that 172,800 simulated seconds (2 days)
/// are processed in about 4 minutes of real time.
/// Each iteration represents 1 minute of simulated time.
pub fn run_simulation(sim_tx: Sender<SimEvent>, log_tx: Sender<LogEvent>) {
    let total_simulated_seconds = 172_800; // 2 days.
    let step_duration_simulated = 60; // Each simulation step is 60 simulated seconds.
    let num_steps = total_simulated_seconds / step_duration_simulated;
    // With a 720× acceleration, each simulated minute takes (60/720) ≈ 83 ms.
    let real_time_per_step = Duration::from_millis((step_duration_simulated * 1000) / 720);

    let mut vehicle_id_counter = 0;

    for step in 0..num_steps {
        let sim_timestamp = step * step_duration_simulated;
        // Determine the time of day (simulate a 24-hour cycle).
        let time_of_day = sim_timestamp % 86_400;
        // Define rush hour periods: 7–9 AM and 4–6 PM.
        let is_rush_hour = (time_of_day >= 25_200 && time_of_day < 32_400) ||
                           (time_of_day >= 57_600 && time_of_day < 64_800);
        // Set an event rate based on time of day (arbitrary values).
        let event_rate = if is_rush_hour { 10 } else { 5 };

        // For each of 4 junctions, simulate events on 3 lanes.
        for junction in 1..=4 {
            for lane in 1..=3 {
                // Simulate vehicle arrival.
                vehicle_id_counter += 1;
                let arrival_event = SimEvent::VehicleArrival {
                    vehicle_id: vehicle_id_counter,
                    junction,
                    lane,
                    timestamp: sim_timestamp,
                };
                sim_tx.send(arrival_event).unwrap();

                // Simulate a vehicle departure for every alternate vehicle.
                if vehicle_id_counter % 2 == 0 {
                    let departure_event = SimEvent::VehicleDeparture {
                        vehicle_id: vehicle_id_counter,
                        junction,
                        lane,
                        timestamp: sim_timestamp,
                    };
                    sim_tx.send(departure_event).unwrap();
                }
            }
            // Simulate a periodic traffic update event per junction.
            let vehicle_count = event_rate * 3; // Rough estimate per junction.
            let update_event = SimEvent::TrafficUpdate {
                junction,
                vehicle_count,
                timestamp: sim_timestamp,
            };
            sim_tx.send(update_event).unwrap();
        }

        // Log the simulation progress.
        let log_message = format!("Simulated minute {} (Simulated Time: {} seconds)", step, sim_timestamp);
        let log_event = LogEvent {
            source: "SimulationEngine".to_string(),
            message: log_message,
            timestamp: sim_timestamp,
        };
        log_tx.send(log_event).unwrap();

        // Sleep for the real time corresponding to this simulation step.
        thread::sleep(real_time_per_step);
    }

    // Log the completion of the simulation.
    let log_event = LogEvent {
        source: "SimulationEngine".to_string(),
        message: "Simulation complete.".to_string(),
        timestamp: total_simulated_seconds,
    };
    log_tx.send(log_event).unwrap();
}
