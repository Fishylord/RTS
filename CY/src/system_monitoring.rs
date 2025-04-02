use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Command, Child};
use tokio::time::{sleep, Duration};
use std::error::Error;
use std::io;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::collections::HashMap;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Welcome to the Traffic Simulation Admin Panel!");

    loop {
        println!("\nChoose an action:");
        println!("1. Start Simulation (Concurrent)");
        println!("2. Show Logs");
        println!("3. Exit");

        let mut choice = String::new();
        io::stdin().read_line(&mut choice)?;
        let choice = choice.trim();

        match choice {
            "1" => {
                // Wipe out any existing JSON logs/configurations from previous runs.
                let files_to_wipe = [
                    "message_log.json", 
                    "sim_config.json", 
                    "light_timings_log.json", 
                    "car_simulation_logs.json"
                ];
                for file in &files_to_wipe {
                    let _ = tokio::fs::write(file, "").await;
                }

                // Prompt for base timing for traffic lights.
                println!("Enter base timing for traffic lights (must be > 15 and < 100 seconds):");
                let mut timing_input = String::new();
                let base_timing: f64;
                loop {
                    timing_input.clear();
                    io::stdin().read_line(&mut timing_input)?;
                    let input_trim = timing_input.trim();
                    if let Ok(val) = input_trim.parse::<f64>() {
                        if val > 15.0 && val < 100.0 {
                            base_timing = val;
                            // Write the timing value to a configuration file.
                            let config = json!({ "base_timing": base_timing });
                            tokio::fs::write("sim_config.json", config.to_string()).await?;
                            break;
                        } else {
                            println!("Invalid value. Base timing must be > 15 and < 100 seconds. Please try again:");
                        }
                    } else {
                        println!("Invalid input. Please enter a numeric value:");
                    }
                }

                println!("Starting components concurrently with base timing: {} seconds...", base_timing);

                // Spawn traffic_light and flow_analyzer as background processes.
                let mut tl_child = spawn_command("traffic_light").await?;
                let mut fa_child = spawn_command("flow_analyzer").await?;
                // Spawn the simulation process and wait for it to finish.
                let mut sim_child = spawn_command("simulation").await?;
                let sim_status = sim_child.wait().await?;
                println!("Simulation finished with status: {}", sim_status);

                // Kill the background processes after simulation ends.
                let _ = tl_child.kill().await;
                let _ = fa_child.kill().await;
            }
            "2" => {
                show_logs_menu().await?;
            }
            "3" => {
                println!("Exiting...");
                break;
            }
            _ => {
                println!("Invalid choice. Please try again.");
            }
        }
    }

    Ok(())
}

async fn spawn_command(binary_name: &str) -> Result<Child, Box<dyn Error>> {
    let mut command = tokio::process::Command::new("cargo");
    command.arg("run").arg("--bin").arg(binary_name);
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    let mut child = command.spawn()?;

    // Spawn background task to print stdout.
    if let Some(stdout) = child.stdout.take() {
        let bin = binary_name.to_string();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                println!("{} (stdout): {}", bin, line);
            }
        });
    }
    // Spawn background task to print stderr.
    if let Some(stderr) = child.stderr.take() {
        let bin = binary_name.to_string();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                println!("{} (stderr): {}", bin, line);
            }
        });
    }
    Ok(child)
}

async fn show_logs_menu() -> Result<(), Box<dyn Error>> {
    loop {
        println!("\n--- Show Logs Menu ---");
        println!("1. Car logs.");
        println!("2. Light_timings.");
        println!("3. Wait_time Heatmap.");
        println!("4. Back.");

        let mut choice = String::new();
        io::stdin().read_line(&mut choice)?;
        let choice = choice.trim();

        match choice {
            "1" => {
                show_car_logs().await?;
            }
            "2" => {
                show_light_timings_logs().await?;
            }
            "3" => {
                println!("Wait_time Heatmap not yet implemented.");
            }
            "4" => break,
            _ => {
                println!("Invalid choice. Please try again.");
            }
        }
    }
    Ok(())
}

#[derive(Deserialize, Debug)]
struct CarRouteData {
    car_id: u32,
    speed: f64,
    spawn_category: String,
    route: Vec<u32>,
}

#[derive(Deserialize, Debug)]
struct CarMetrics {
    id: u32,
    wait_time: f64,
    drive_time: f64,
    total_time: f64,
}

#[derive(Deserialize, Debug)]
struct CarLog {
    car_route: CarRouteData,
    metrics: CarMetrics,
}

async fn show_car_logs() -> Result<(), Box<dyn Error>> {
    let file_path = "car_simulation_logs.json";
    if !Path::new(file_path).exists() {
        println!("No car simulation logs found.");
        return Ok(());
    }
    
    let content = tokio::fs::read_to_string(file_path).await?;
    let mut logs: Vec<CarLog> = Vec::new();
    
    for line in content.lines() {
        if line.trim().is_empty() { continue; }
        match serde_json::from_str::<CarLog>(line) {
            Ok(log) => logs.push(log),
            Err(e) => println!("Failed to parse line: {} ({})", line, e),
        }
    }
    
    logs.sort_by(|a, b| a.car_route.car_id.cmp(&b.car_route.car_id));
    
    let page_size = 5;
    let mut page_index: usize = 0;
    
    loop {
        let total_pages = if logs.len() % page_size == 0 {
            logs.len() / page_size
        } else {
            logs.len() / page_size + 1
        };
        
        println!("\n============== Car Logs (Page {}/{}) =============================", page_index + 1, total_pages);
        let start = page_index * page_size;
        let end = std::cmp::min(start + page_size, logs.len());
        for log in &logs[start..end] {
            println!(
                "Car id: {} | Speed: {:.2} | Route: {:?} | wait_time: {:.2} | drive_time: {:.2} | total_time: {:.2}",
                log.car_route.car_id,
                log.car_route.speed,
                log.car_route.route,
                log.metrics.wait_time,
                log.metrics.drive_time,
                log.metrics.total_time,
            );
        }
        println!("============================================================");
        
        println!("Choices:");
        println!("1. Next page");
        println!("2. Previous page");
        println!("3. Specific Car id data (or page jump in format X/Y)");
        println!("4. Exit Car logs");
        
        let mut nav = String::new();
        io::stdin().read_line(&mut nav)?;
        let nav = nav.trim();
        
        match nav {
            "1" => {
                if page_index + 1 < total_pages {
                    page_index += 1;
                } else {
                    println!("Already at the last page.");
                }
            }
            "2" => {
                if page_index > 0 {
                    page_index -= 1;
                } else {
                    println!("Already at the first page.");
                }
            }
            "3" => {
                println!("Enter Car id (or page jump in format X/Y):");
                let mut id_input = String::new();
                io::stdin().read_line(&mut id_input)?;
                let id_input = id_input.trim();
                if id_input.contains('/') {
                    let parts: Vec<&str> = id_input.split('/').collect();
                    if parts.len() == 2 {
                        if let (Ok(num), Ok(divisor)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                            if divisor == 0 {
                                println!("Divisor cannot be zero.");
                                continue;
                            }
                            let target_page = num / divisor;
                            if target_page == 0 || target_page > total_pages {
                                println!("Invalid page jump.");
                                continue;
                            }
                            page_index = target_page - 1;
                        } else {
                            println!("Invalid input format.");
                        }
                    } else {
                        println!("Invalid input format.");
                    }
                } else {
                    if let Ok(target_id) = id_input.parse::<u32>() {
                        if let Some(pos) = logs.iter().position(|log| log.car_route.car_id == target_id) {
                            page_index = pos / page_size;
                        } else {
                            println!("Invalid car id.");
                        }
                    } else {
                        println!("Invalid input.");
                    }
                }
            }
            "4" => break,
            _ => println!("Invalid choice. Please try again."),
        }
    }
    
    Ok(())
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct JunctionTimingLog {
    junction_id: u32,
    groups: Vec<Vec<u32>>,
    timings: Vec<f64>,
    timestamp: u64,
}

async fn show_light_timings_logs() -> Result<(), Box<dyn Error>> {
    let file_path = "light_timings_log.json";
    if !Path::new(file_path).exists() {
        println!("No light timings logs found.");
        return Ok(());
    }
    
    let content = tokio::fs::read_to_string(file_path).await?;
    let mut items: Vec<(u32, String)> = Vec::new();
    
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        if let Ok(mut arr) = serde_json::from_str::<Vec<JunctionTimingLog>>(trimmed) {
            arr.sort_by(|a, b| a.junction_id.cmp(&b.junction_id));
            if let Some(first) = arr.first() {
                let sorted_json = serde_json::to_string(&arr)?;
                items.push((first.junction_id, sorted_json));
            }
        } else {
            println!("Failed to parse line: {}", trimmed);
        }
    }
    
    items.sort_by(|a, b| a.0.cmp(&b.0));
    let sorted_items: Vec<String> = items.into_iter().map(|(_, s)| s).collect();
    
    let page_size = 1;
    let mut page_index: usize = 0;
    
    loop {
        let total_pages = if sorted_items.len() % page_size == 0 {
            sorted_items.len() / page_size
        } else {
            sorted_items.len() / page_size + 1
        };
        
        println!("\n============== Light Timings Logs (Page {}/{}) =============================", page_index + 1, total_pages);
        let start = page_index * page_size;
        let end = std::cmp::min(start + page_size, sorted_items.len());
        for item in &sorted_items[start..end] {
            println!("{}", item);
        }
        println!("============================================================");
        
        println!("Choices:");
        println!("1. Next page");
        println!("2. Previous page");
        println!("3. Exit Light Timings logs");
        
        let mut nav = String::new();
        io::stdin().read_line(&mut nav)?;
        let nav = nav.trim();
        
        match nav {
            "1" => {
                if page_index + 1 < total_pages {
                    page_index += 1;
                } else {
                    println!("Already at the last page.");
                }
            }
            "2" => {
                if page_index > 0 {
                    page_index -= 1;
                } else {
                    println!("Already at the first page.");
                }
            }
            "3" => break,
            _ => println!("Invalid choice. Please try again."),
        }
    }
    
    Ok(())
}
