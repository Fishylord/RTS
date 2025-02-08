mod simulation;
mod flow_analyzer;
mod traffic_light;
mod system_monitoring;

use std::io;
use std::sync::mpsc;
use std::thread;

fn main() {
    println!("=== Traffic Simulation Monitoring CLI ===");
    println!("Commands: 'start' to begin simulation, 'exit' to quit.");

    let mut input = String::new();
    loop {
        input.clear();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read input");
        let command = input.trim();
        if command.eq_ignore_ascii_case("start") {
            // This function spawns all threads for the simulation
            start_simulation();
            println!("Simulation completed.\nType 'start' to run again, or 'exit' to quit.");
        } else if command.eq_ignore_ascii_case("exit") {
            println!("Exiting.");
            break;
        } else {
            println!("Unknown command. Available commands: start, exit");
        }
    }
}

/// Spawns the simulation, flow analyzer, traffic lights, and monitoring threads,
/// and then waits for all threads to complete.
fn start_simulation() {
    // Create channels for inter-component communication.
    // sim_tx/sim_rx: simulation events from SimulationEngine to FlowAnalyzer.
    let (sim_tx, sim_rx) = mpsc::channel::<simulation::SimEvent>();
    // fa_tx/fa_rx: recommendations from FlowAnalyzer to TrafficLight.
    let (fa_tx, fa_rx) = mpsc::channel::<flow_analyzer::Recommendation>();
    // log_tx/log_rx: log events from all components to the Monitoring system.
    let (log_tx, log_rx) = mpsc::channel::<system_monitoring::LogEvent>();

    // Spawn Simulation Engine thread.
    let sim_thread = thread::spawn({
        let sim_tx = sim_tx.clone();
        let log_tx = log_tx.clone();
        move || {
            simulation::run_simulation(sim_tx, log_tx);
        }
    });

    // Spawn Traffic Flow Analyzer thread.
    let fa_thread = thread::spawn({
        let log_tx = log_tx.clone();
        move || {
            flow_analyzer::run_flow_analyzer(sim_rx, fa_tx, log_tx);
        }
    });

    // Spawn Traffic Light Control thread.
    let tl_thread = thread::spawn({
        let log_tx = log_tx.clone();
        move || {
            traffic_light::run_traffic_lights(fa_rx, log_tx);
        }
    });

    // Spawn System Monitoring and Reporting thread.
    let sm_thread = thread::spawn(move || {
        system_monitoring::run_monitoring(log_rx);
    });

    // Wait for threads to finish.
    sim_thread.join().unwrap();
    // When the simulation finishes, the channels will eventually be closed.
    // (A more complete system would include explicit shutdown signals.)
    fa_thread.join().unwrap();
    tl_thread.join().unwrap();
    sm_thread.join().unwrap();
}
