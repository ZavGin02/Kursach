use crossterm::{
    event,
    style::{Print, Stylize},
    terminal, ExecutableCommand,
};
use log::{info, error, LevelFilter};
use simplelog::{Config, WriteLogger, TermLogger, TerminalMode, CombinedLogger};
use std::fs::File;
use std::io::{self, Write};
use std::process::Command;
use std::str;
use std::time::Duration;

fn init_logger() {
    CombinedLogger::init(
        vec![
            TermLogger::new(
                LevelFilter::Info,
                Config::default(),
                TerminalMode::Mixed,
                simplelog::ColorChoice::Auto,
            ),
            WriteLogger::new(
                LevelFilter::Debug,
                Config::default(),
                File::create("gpu_temp_reader.log").unwrap(),
            ),
        ]
    ).unwrap();
}

fn get_gpu_temperature() -> Result<f32, String> {
    let output = Command::new("nvidia-smi")
        .arg("--query-gpu=temperature.gpu")
        .arg("--format=csv,noheader,nounits")
        .output()
        .map_err(|e| format!("Failed to execute command: {}", e))?;

    if !output.status.success() {
        return Err(format!("Command failed with status: {}, stderr: {}", output.status, String::from_utf8_lossy(&output.stderr)));
    }

    let temp_str = str::from_utf8(&output.stdout).map_err(|e| format!("Failed to parse output: {}", e))?;
    temp_str.trim().parse::<f32>().map_err(|e| format!("Failed to parse temperature: {}", e))
}

fn get_gpu_load() -> Result<f32, String> {
    let output = Command::new("nvidia-smi")
        .arg("--query-gpu=utilization.gpu")
        .arg("--format=csv,noheader,nounits")
        .output()
        .map_err(|e| format!("Failed to execute command: {}", e))?;

    if !output.status.success() {
        return Err(format!("Command failed with status: {}, stderr: {}", output.status, String::from_utf8_lossy(&output.stderr)));
    }

    let load_str = str::from_utf8(&output.stdout).map_err(|e| format!("Failed to parse output: {}", e))?;
    load_str.trim().parse::<f32>().map_err(|e| format!("Failed to parse load: {}", e))
}

fn get_gpu_model() -> Result<String, String> {
    let output = Command::new("nvidia-smi")
        .arg("--query-gpu=name")
        .arg("--format=csv,noheader")
        .output()
        .map_err(|e| format!("Failed to execute command: {}", e))?;

    if !output.status.success() {
        return Err(format!("Command failed with status: {}, stderr: {}", output.status, String::from_utf8_lossy(&output.stderr)));
    }

    let model_str = str::from_utf8(&output.stdout).map_err(|e| format!("Failed to parse output: {}", e))?;
    Ok(model_str.trim().to_string())
}

fn main() -> io::Result<()> {
    init_logger();
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(terminal::Clear(terminal::ClearType::All))?;
    info!("Starting gpu_temp_reader");

    loop {
        match get_gpu_temperature() {
            Ok(temp) => {
                let load = match get_gpu_load() {
                    Ok(l) => l,
                    Err(e) => {
                        error!("Failed to get GPU load: {}", e);
                        0.0
                    }
                };
                let model = match get_gpu_model() {
                    Ok(m) => m,
                    Err(e) => {
                        error!("Failed to get GPU model: {}", e);
                        "Unknown".to_string()
                    }
                };
                info!("GPU: {} Temperature: {} °C, Load: {}%", model, temp, load);

                if temp > 70.0 {
                    stdout.execute(Print(
                        format!("\rGPU: {} Temperature: {} °C, Load: {}%", model, temp.to_string().red(), load)
                    ))?;
                } else {
                    stdout.execute(Print(
                        format!("\rGPU: {} Temperature: {} °C, Load: {}%", model, temp, load)
                    ))?;
                }

                stdout.flush()?;
            }
            Err(e) => {
                error!("Failed to get GPU temperature: {}", e);
                stdout.execute(Print(format!("\rError: {}", e)))?;
                stdout.flush()?;
            }
        }

        if event::poll(Duration::from_secs(1))? {
            if let event::Event::Key(event) = event::read()? {
                if event.code == event::KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    terminal::disable_raw_mode()?;
    println!("\nProgram terminated.");
    info!("Exiting gpu_temp_reader");
    Ok(())
}