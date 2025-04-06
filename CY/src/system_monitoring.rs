use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Command, Child};
use tokio::time::{sleep, Duration};
use std::error::Error;
use std::io;
use serde::{Serialize, Deserialize};
use std::path::Path;
use serde_json::Value;
use std::process::Stdio;
use plotters::prelude::*;
use plotters::style::HSLColor;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
struct JunctionTimingLog {
    junction_id: u32,
    groups: Vec<Vec<u32>>,
    timings: Vec<f64>,
    timestamp: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Welcome to the Traffic Simulation Admin Panel!");

    loop {
        println!("\nChoose an action:");
        println!("1. Start Simulation (Progress Bar Mode)");
        println!("2. Show Logs");
        println!("3. Show Performance Metrics");
        println!("4. Exit");

        let mut choice = String::new();
        io::stdin().read_line(&mut choice)?;
        let choice = choice.trim();

        match choice {
            "1" => start_simulation().await?,
            "2" => show_logs_menu().await?,
            "3" => show_performance_metrics_menu().await?,
            "4" => {
                println!("Exiting...");
                break;
            }
            _ => println!("Invalid choice. Please try again."),
        }
    }
    Ok(())
}

// --------------------- Simulation Starter ---------------------
async fn start_simulation() -> Result<(), Box<dyn Error>> {
    // Reset log and configuration files.
    let files_to_wipe = [
        "message_log.json", 
        "sim_config.json", 
        "light_timings_log.json", 
        "car_simulation_logs.json",
        "progress.txt",
        "final_metrics.txt",
        "performance_metrics.json",
    ];
    for file in &files_to_wipe {
        let _ = tokio::fs::write(file, "").await;
    }

    // Prompt for base timing.
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
                break;
            } else {
                println!("Invalid value. Base timing must be > 15 and < 100 seconds. Please try again:");
            }
        } else {
            println!("Invalid input. Please enter a numeric value:");
        }
    }

    // Prompt for simulation relative speed.
    println!("Enter simulation relative speed (acceleration factor) (must be >= 100 and <= 500, recommended 250 for a 2-minute simulation):");
    let mut accel_input = String::new();
    let acceleration: f64;
    loop {
        accel_input.clear();
        io::stdin().read_line(&mut accel_input)?;
        let input_trim = accel_input.trim();
        if let Ok(val) = input_trim.parse::<f64>() {
            if val >= 100.0 && val <= 500.0 {
                acceleration = val;
                break;
            } else {
                println!("Invalid value. Acceleration must be between 100 and 500. Please try again:");
            }
        } else {
            println!("Invalid input. Please enter a numeric value:");
        }
    }

    // Write configuration to file.
    let config = serde_json::json!({ "base_timing": base_timing, "acceleration": acceleration });
    tokio::fs::write("sim_config.json", config.to_string()).await?;

    println!("Starting simulation components with base timing: {} seconds and acceleration: {}.", base_timing, acceleration);

    // Spawn simulation components.
    let mut tl_child = spawn_command("traffic_light").await?;
    let mut fa_child = spawn_command("flow_analyzer").await?;
    let mut sim_child = spawn_command("simulation").await?;

    // Display progress bar.
    let progress_handle = tokio::spawn(async {
        loop {
            sleep(Duration::from_secs(1)).await;
            if Path::new("progress.txt").exists() {
                if let Ok(progress) = tokio::fs::read_to_string("progress.txt").await {
                    print!("\r{}", progress);
                    let _ = io::Write::flush(&mut io::stdout());
                }
            }
        }
    });

    // Wait for simulation to finish.
    let sim_status = sim_child.wait().await?;
    progress_handle.abort();

    // Display final metrics.
    if Path::new("final_metrics.txt").exists() {
        let final_metrics = tokio::fs::read_to_string("final_metrics.txt").await?;
        println!("\nSimulation Finished. Final Metrics:");
        println!("{}", final_metrics);
    } else {
        println!("\nSimulation Finished, but final metrics were not found.");
    }
    println!("Simulation finished with status: {}", sim_status);

    // Kill background processes.
    let _ = tl_child.kill().await;
    let _ = fa_child.kill().await;

    Ok(())
}

async fn spawn_command(binary_name: &str) -> Result<Child, Box<dyn Error>> {
    let mut command = Command::new("cargo");
    command.arg("run").arg("--bin").arg(binary_name);
    command.stdout(Stdio::null());
    command.stderr(Stdio::null());
    let child = command.spawn()?;
    Ok(child)
}

// --------------------- Logs Menu ---------------------
async fn show_logs_menu() -> Result<(), Box<dyn Error>> {
    loop {
        println!("\n--- Show Logs Menu ---");
        println!("1. Car logs.");
        println!("2. Light Timings logs.");
        println!("3. Wait Time Heatmap (Not yet implemented).");
        println!("4. Back.");

        let mut choice = String::new();
        io::stdin().read_line(&mut choice)?;
        let choice = choice.trim();

        match choice {
            "1" => show_car_logs().await?,
            "2" => show_light_timings_logs().await?,
            "3" => show_wait_time_heatmap().await?,
            "4" => break,
            _ => println!("Invalid choice. Please try again."),
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
    // Extra fields (like lanes) in the log are ignored.
}

#[derive(Deserialize, Debug)]
struct CarMetrics {
    id: u32,
    wait_time: f64,
    drive_time: f64,
    simulated_total_time: f64,
    wall_time: f64,
    lane_queue_overhead: f64,
    cpu_processing_time: f64,
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
                "Car id: {} | Speed: {:.2} | Route: {:?} | Wait: {:.2}s | Drive: {:.2}s | Total: {:.2}s",
                log.car_route.car_id,
                log.car_route.speed,
                log.car_route.route,
                log.metrics.wait_time,
                log.metrics.drive_time,
                log.metrics.simulated_total_time,
            );
        }
        println!("============================================================");
        
        println!("Choices:");
        println!("1. Next page");
        println!("2. Previous page");
        println!("3. Specific Car id (or page jump in format X/Y)");
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

// --------------------- Performance Metrics Menu ---------------------
async fn show_performance_metrics_menu() -> Result<(), Box<dyn Error>> {
    loop {
        println!("\n--- Performance Metrics Menu ---");
        println!("1. Flow Analyzer Metrics");
        println!("2. Simulation Metrics");
        println!("3. Traffic Light Metrics");
        println!("4. Return");

        let mut choice = String::new();
        io::stdin().read_line(&mut choice)?;
        let choice = choice.trim();

        match choice {
            "1" => show_flow_analyzer_metrics().await?,
            "2" => show_simulation_metrics().await?,
            "3" => show_traffic_light_metrics().await?,
            "4" => break,
            _ => println!("Invalid choice, please try again."),
        }
    }
    Ok(())
}

async fn show_flow_analyzer_metrics() -> Result<(), Box<dyn Error>> {
    let file_path = "flow_analyzer_metrics.json";
    if !Path::new(file_path).exists() {
        println!("No Flow Analyzer performance metrics found.");
        return Ok(());
    }
    let content = tokio::fs::read_to_string(file_path).await?;
    if content.trim().is_empty() {
        println!("No Flow Analyzer performance metrics found.");
        return Ok(());
    }
    let parsed: Value = serde_json::from_str(&content)?;
    println!("\n--- Flow Analyzer Metrics ---");
    println!("{}", serde_json::to_string_pretty(&parsed)?);
    println!("Press Enter to return.");
    let mut temp = String::new();
    io::stdin().read_line(&mut temp)?;
    Ok(())
}

async fn show_simulation_metrics() -> Result<(), Box<dyn Error>> {
    let file_path = "performance_metrics.json";
    if !Path::new(file_path).exists() {
        println!("No Simulation performance metrics found.");
        return Ok(());
    }
    let content = tokio::fs::read_to_string(file_path).await?;
    if content.trim().is_empty() {
        println!("No Simulation performance metrics found.");
        return Ok(());
    }
    let metrics: Vec<Value> = serde_json::from_str(&content)?;
    if metrics.is_empty() {
        println!("No Simulation performance metrics found.");
        return Ok(());
    }
    let mut sum_wait = 0.0;
    let mut sum_drive = 0.0;
    let mut sum_simulated = 0.0;
    let mut sum_wall = 0.0;
    let mut sum_queue = 0.0;
    let mut sum_cpu = 0.0;
    let mut count = 0.0;
    for m in metrics.iter() {
        let wait_time = m.get("wait_time").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let drive_time = m.get("drive_time").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let simulated_total_time = m.get("simulated_total_time").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let wall_time = m.get("wall_time").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let lane_queue_overhead = m.get("lane_queue_overhead").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let cpu_processing_time = m.get("cpu_processing_time").and_then(|v| v.as_f64()).unwrap_or(0.0);
        sum_wait += wait_time;
        sum_drive += drive_time;
        sum_simulated += simulated_total_time;
        sum_wall += wall_time;
        sum_queue += lane_queue_overhead;
        sum_cpu += cpu_processing_time;
        count += 1.0;
    }
    if count == 0.0 {
        println!("No valid Simulation metrics found.");
        return Ok(());
    }
    let avg_wait = sum_wait / count;
    let avg_drive = sum_drive / count;
    let avg_simulated = sum_simulated / count;
    let avg_wall = sum_wall / count;
    let avg_queue = sum_queue / count;
    let avg_cpu = sum_cpu / count;
    println!("\n--- Simulation Metrics Averages ---");
    println!("Average Wait Time: {:.2}s", avg_wait);
    println!("Average Drive Time: {:.2}s", avg_drive);
    println!("Average Simulated Total Time: {:.2}s", avg_simulated);
    println!("Average Wall-Clock Time: {:.2}s", avg_wall);
    println!("Average Lane Queue Overhead: {:.2}s", avg_queue);
    println!("Average CPU Processing Time: {:.2}s", avg_cpu);
    println!("Press Enter to return.");
    let mut temp = String::new();
    io::stdin().read_line(&mut temp)?;
    Ok(())
}

async fn show_traffic_light_metrics() -> Result<(), Box<dyn Error>> {
    let file_path = "traffic_light_metrics.json";
    if !Path::new(file_path).exists() {
        println!("No Traffic Light performance metrics found.");
        return Ok(());
    }
    let content = tokio::fs::read_to_string(file_path).await?;
    if content.trim().is_empty() {
        println!("No Traffic Light performance metrics found.");
        return Ok(());
    }
    let parsed: Value = serde_json::from_str(&content)?;
    println!("\n--- Traffic Light Metrics ---");
    println!("{}", serde_json::to_string_pretty(&parsed)?);
    println!("Press Enter to return.");
    let mut temp = String::new();
    io::stdin().read_line(&mut temp)?;
    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
struct LaneWaitingTime {
    lane_id: u32,
    average_waiting_time: f64,
    timestamp: u64,
}

async fn show_wait_time_heatmap() -> Result<(), Box<dyn Error>> {
    let lane_coordinates: HashMap<u32, (usize, usize)> = HashMap::from([
        (1000, (1, 12)), (1001, (4, 12)), (1002, (7, 12)), (1003, (12, 10)),
        (1004, (0, 8)), (1005, (12, 4)), (1006, (0, 2)), (1007, (2, 0)),
        (1008, (8, 0)), (1009, (11, 0)), (1010, (0, 10)), (1011, (5, 12)),
        (1012, (11, 12)), (1013, (0, 7)), (1014, (12, 5)), (1015, (7, 0)),
        (1016, (12, 2)), (1017, (10, 0)), (1018, (3, 10)), (1019, (6, 10)),
        (1020, (9, 10)), (1021, (11, 9)), (1022, (1, 9)), (1023, (3, 7)),
        (1024, (2, 6)), (1025, (3, 8)), (1026, (5, 9)), (1027, (4, 9)),
        (1028, (6, 7)), (1029, (6, 8)), (1030, (7, 9)), (1031, (9, 7)),
        (1032, (9, 8)), (1033, (11, 6)), (1034, (3, 4)), (1035, (2, 3)),
        (1036, (3, 5)), (1037, (6, 4)), (1038, (5, 3)), (1039, (6, 5)),
        (1040, (7, 6)), (1041, (8, 3)), (1042, (10, 6)), (1043, (11, 3)),
        (1044, (3, 2)), (1045, (4, 3)), (1046, (6, 1)), (1047, (6, 2)),
        (1048, (7, 3)), (1049, (9, 1)), (1050, (10, 3)), (1051, (9, 2)),
    ]);

    let mut heatmap_data: Vec<Vec<Option<f64>>> = vec![vec![None; 13]; 13];

    let log_file_path = "message_log.json";  // Replace with your file path
    let content = tokio::fs::read_to_string(log_file_path).await?;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }

        if let Ok(lane_data) = serde_json::from_str::<LaneWaitingTime>(trimmed) {

            if let Some(&(x, y)) = lane_coordinates.get(&lane_data.lane_id) {
                heatmap_data[y][x] = Some(lane_data.average_waiting_time);
            } else {
                println!("Lane ID {} not found in coordinate map.", lane_data.lane_id);
            }
        } else {
            println!("Failed to parse line: {}", line);

        }
    }

    let rows = heatmap_data.len();
    let cols = heatmap_data[0].len();

    let root = BitMapBackend::new("heatmap.png", (cols as u32 * 50, rows as u32 * 50)).into_drawing_area(); // Adjust size
    root.fill(&WHITE)?;

    let x_range = 0..cols;
    let y_range = 0..rows;

    let mut chart = ChartBuilder::on(&root)
        .caption("Wait Time Heatmap", ("Arial", 20))
        .margin(10)
        .set_label_area_size(LabelAreaPosition::Left, 40)
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .build_cartesian_2d(x_range.clone(), y_range.clone())?;

    chart
        .configure_mesh()
        .x_desc("X Axis")
        .y_desc("Y Axis")
        .disable_mesh()
        .draw()?;


    let max_wait_time = heatmap_data.iter().flatten().filter_map(|x| *x).fold(0.0f64, f64::max); // Calculate the maximum value for normalization



    for (y, row) in heatmap_data.iter().enumerate() {
        for (x, value) in row.iter().enumerate() {
            if let Some(value) = value {
                let normalized = *value / max_wait_time; // Normalize to 0..1 range using actual max value
                let color = HSLColor(240.0 / 360.0 - 240.0 / 360.0 * normalized, 1.0, 0.5); // Use HSL to create nicer gradient
                chart.draw_series(std::iter::once(Rectangle::new(
                    [(x, y), (x + 1, y + 1)],
                    color.filled(),
                )))?;
            } else {
                chart.draw_series(std::iter::once(Rectangle::new(
                    [(x, y), (x + 1, y + 1)],
                    RGBColor(200, 200, 200).filled(), // Light gray for missing data
                )))?;
            }
        }
    }

    root.present()?;

    Ok(())
}
