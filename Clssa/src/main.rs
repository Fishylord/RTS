mod simulation;
mod traffic_light;
mod system_monitoring;
mod lanes;
mod flow_analyzer;

use std::{collections::HashMap, sync::Arc};
use std::thread;
use std::sync::mpsc;

use simulation::{run_simulation};
use traffic_light::{run_traffic_lights, initialize_traffic_lights, TrafficLightMap};
use system_monitoring::LogEvent;
use flow_analyzer::{run_flow_analyzer,Recommendation};

fn main() {
    println!("=== Real-Time 16-Junction Traffic Simulation ===");

    // Initialize traffic lights for all lanes that require control.
    // All lights are initialized to Red so that not all are green at startup.
    let traffic_lights: TrafficLightMap = initialize_traffic_lights();

    //channel for recommendation
    let (analyzer_tx, analyzer_rx) = mpsc::channel::<HashMap<u32,u32>>();
    let (rec_tx, rec_rx) = mpsc::channel::<Recommendation>();

    // Channel for log events.
    let (log_tx, log_rx) = mpsc::channel::<LogEvent>();

    // Start the Traffic Light Controller.
    // This call spawns a thread per junction internally.   
    let tl_traffic_lights = Arc::clone(&traffic_lights);
    let tl_log_tx = log_tx.clone();
    thread::spawn(move || {
        run_traffic_lights(tl_traffic_lights, tl_log_tx, rec_rx);
    });

    //start the flow analyzer thread
    thread::spawn(move || {
        run_flow_analyzer(analyzer_rx,rec_tx);
    });


    // Spawn the Simulation Engine thread (which spawns 30 car threads).
    let sim_traffic_lights = Arc::clone(&traffic_lights);
    let simulation_handle = thread::spawn(move || {
        run_simulation(sim_traffic_lights, log_tx, analyzer_tx);
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