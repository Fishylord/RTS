use std::env;
use std::process::Command;

mod simulation;
mod traffic_light;
mod system_monitoring;
mod lanes;
mod flow_analyzer;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "simulation" => {
                let traffic_lights = traffic_light::initialize_traffic_lights();
                simulation::run_simulation(traffic_lights);
            },
            "traffic_light" => {
                let traffic_lights = traffic_light::initialize_traffic_lights();
                traffic_light::run_traffic_lights(traffic_lights);
            },
            "analyzer" => {
                flow_analyzer::run_flow_analyzer();
            },
            "monitoring" => {
                system_monitoring::run_monitoring();
            },
            _ => {
                eprintln!("Unknown component: {}", args[1]);
            }
        }
    } else {
        // If no argument is given, spawn all components as separate processes.
        let current_exe = env::current_exe().expect("Failed to get current executable");
        let components = ["simulation", "traffic_light", "analyzer", "monitoring"];
        let mut children = Vec::new();
        
        for comp in &components {
            let child = Command::new(&current_exe)
                .arg(comp)
                .spawn()
                .expect(&format!("Failed to spawn {} process", comp));
            println!("Spawned {} process", comp);
            children.push(child);
        }
        
        for mut child in children {
            child.wait().expect("Child process encountered an error");
        }
    }
}
