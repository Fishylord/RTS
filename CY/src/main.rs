mod simulation;
mod traffic_light;
mod system_monitoring;

use std::sync::Arc;
use std::thread;
use std::sync::mpsc;

use simulation::run_simulation;
use traffic_light::{run_traffic_lights, initialize_traffic_lights, TrafficLightMap};
use system_monitoring::LogEvent;

fn main() {
    println!("=== Real-Time 16-Junction Traffic Simulation ===");

    // Initialize traffic lights for all lanes that require control.
    // All lights are initialized to Red so that not all are green at startup.
    let traffic_lights: TrafficLightMap = initialize_traffic_lights();

    // Channel for log events.
    let (log_tx, log_rx) = mpsc::channel::<LogEvent>();

    // Start the Traffic Light Controller.
    // This call spawns a thread per junction internally.
    let tl_traffic_lights = Arc::clone(&traffic_lights);
    let tl_log_tx = log_tx.clone();
    thread::spawn(move || {
        run_traffic_lights(tl_traffic_lights, tl_log_tx);
    });

    // Spawn the Simulation Engine thread (which spawns 30 car threads).
    let sim_traffic_lights = Arc::clone(&traffic_lights);
    let simulation_handle = thread::spawn(move || {
        run_simulation(sim_traffic_lights, log_tx);
    });

    // Spawn the System Monitoring thread.
    let _monitoring_handle = thread::spawn(move || {
        system_monitoring::run_monitoring(log_rx);
    });

    simulation_handle.join().unwrap();
    // Give some time for pending log messages.
    thread::sleep(std::time::Duration::from_secs(1));
    println!("Simulation complete. Exiting.");
}
