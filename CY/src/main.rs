mod simulation;
mod traffic_light;
mod system_monitoring;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::thread;
use std::sync::mpsc;

use simulation::run_simulation;
use traffic_light::run_traffic_lights;
use system_monitoring::LogEvent;

fn main() {
    println!("=== Real-Time Traffic Simulation ===");

    // Create a shared traffic light state for intersections 1–4.
    let traffic_lights = Arc::new(Mutex::new({
        let mut map = HashMap::new();
        // Initialize each intersection with NSGreen.
        for id in 1..=4 {
            map.insert(id, traffic_light::LightState::NSGreen);
        }
        map
    }));

    // Create a channel for logging events.
    let (log_tx, log_rx) = mpsc::channel::<LogEvent>();

    // Spawn the Traffic Light Controller thread.
    let tl_lights = Arc::clone(&traffic_lights);
    let tl_log_tx = log_tx.clone();
    let _traffic_light_handle = thread::spawn(move || {
        run_traffic_lights(tl_lights, tl_log_tx);
    });

    // Spawn the Simulation Engine thread (which spawns 30 car threads).
    let simulation_handle = thread::spawn(move || {
        run_simulation(traffic_lights, log_tx);
    });

    // Spawn the System Monitoring thread to print logs.
    let monitoring_handle = thread::spawn(move || {
        system_monitoring::run_monitoring(log_rx);
    });

    // Wait for the simulation to complete.
    simulation_handle.join().unwrap();

    // (In a complete system we would signal the traffic light thread to shut down.)
    // Sleep briefly so all log messages get printed.
    thread::sleep(std::time::Duration::from_secs(1));
    println!("Simulation complete. Exiting.");
}
