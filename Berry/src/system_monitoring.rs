use std::error::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use std::io;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Welcome to the Traffic Simulation Admin Panel!");

    loop {
        println!("\nChoose an action:");
        println!("1. Start Simulation (Concurrent)");
        println!("2. Show Logs (Not Implemented)");
        println!("3. Exit");

        let mut choice = String::new();
        io::stdin().read_line(&mut choice)?;
        let choice = choice.trim();

        match choice {
            "1" => {
                println!("Starting components concurrently...");

                tokio::try_join!(
                    run_command("flow_analyzer"),
                    run_command("traffic_light"),
                    run_command("simulation")
                )?;
            }
            "2" => {
                println!("Log viewing not yet implemented.");
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

async fn run_command(binary_name: &str) -> Result<(), Box<dyn Error>> {
    let mut command = Command::new("cargo");
    command.arg("run").arg("--bin").arg(binary_name);

    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());

    let mut child = command.spawn()?;
    let stdout = child.stdout.take().ok_or_else(|| format!("Failed to capture stdout for {}", binary_name))?;
    let stderr = child.stderr.take().ok_or_else(|| format!("Failed to capture stderr for {}", binary_name))?;    

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    tokio::join!(
        async {
            while let Some(line) = stdout_reader.next_line().await.unwrap_or_else(|_| None) {
                println!("{} (stdout): {}", binary_name, line);
            }
        },
        async {
            while let Some(line) = stderr_reader.next_line().await.unwrap_or_else(|_| None) {
            }
        }
    ); // End of tokio::join!

    let status = child.wait().await?;

    if !status.success() {
        return Err(format!("{} exited with error: {}", binary_name, status).into());
    }

    Ok(())
}
