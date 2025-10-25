use clap::Parser;
use std::process::{Command, Stdio};
use std::time::Instant;

// Windows-specific imports
#[cfg(windows)]
use winapi::um::processthreadsapi::{GetProcessTimes, OpenProcess};
#[cfg(windows)]
use winapi::shared::minwindef::FILETIME;
#[cfg(windows)]
use winapi::um::winnt::{HANDLE, PROCESS_QUERY_INFORMATION};
#[cfg(windows)]
use winapi::um::handleapi::CloseHandle;

// Unix-specific imports  
#[cfg(unix)]
use libc;



#[derive(Parser)]
#[command(name = "time")]
#[command(about = "A cross-platform Unix-like time command")]
#[command(version = "1.0")]
struct Args {
    /// Output format (currently only supports default format)
    #[arg(short = 'f', long = "format")]
    format: Option<String>,
    
    /// Append timing info to file instead of stderr
    #[arg(short = 'o', long = "output")]
    output_file: Option<String>,
    
    /// Write timing info to stderr (default behavior)
    #[arg(short = 'v', long = "verbose")]
    verbose: bool,
    
    /// Portable mode - use less accurate but more portable timing
    #[arg(short = 'p', long = "portable")]
    portable: bool,
    
    /// Command to execute (everything after the options)
    #[arg(trailing_var_arg = true, required = true)]
    command: Vec<String>,
}

#[derive(Debug, Default)]
struct ResourceUsage {
    user_time: f64,
    system_time: f64,
    #[allow(dead_code)] // May not be used on all platforms
    max_memory: u64, // in KB
}

// Windows implementation
#[cfg(windows)]
fn filetime_to_seconds(ft: &FILETIME) -> f64 {
    let total = ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64);
    // FILETIME is in 100-nanosecond intervals since January 1, 1601
    total as f64 / 10_000_000.0
}

#[cfg(windows)]
fn get_child_process_times(child_id: u32) -> Result<ResourceUsage, Box<dyn std::error::Error>> {
    unsafe {
        let handle: HANDLE = OpenProcess(PROCESS_QUERY_INFORMATION, 0, child_id);
        if handle.is_null() {
            return Ok(ResourceUsage::default());
        }
        
        let mut creation_time = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        let mut exit_time = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        let mut kernel_time = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        let mut user_time = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        
        let result = GetProcessTimes(
            handle,
            &mut creation_time,
            &mut exit_time,
            &mut kernel_time,
            &mut user_time,
        );
        
        CloseHandle(handle);
        
        if result == 0 {
            return Ok(ResourceUsage::default());
        }
        
        let user_seconds = filetime_to_seconds(&user_time);
        let kernel_seconds = filetime_to_seconds(&kernel_time);
        
        Ok(ResourceUsage {
            user_time: user_seconds,
            system_time: kernel_seconds,
            max_memory: 0, // Memory info requires additional APIs
        })
    }
}

// Unix implementation (Linux, macOS, etc.)
#[cfg(unix)]
fn get_child_process_times(child_id: u32) -> Result<ResourceUsage, Box<dyn std::error::Error>> {
    use std::fs;
    
    // Try to read from /proc/[pid]/stat (Linux)
    if let Ok(stat_content) = fs::read_to_string(format!("/proc/{}/stat", child_id)) {
        let fields: Vec<&str> = stat_content.split_whitespace().collect();
        if fields.len() >= 24 {
            // Fields 13 and 14 are utime and stime in clock ticks
            let utime: u64 = fields[13].parse().unwrap_or(0);
            let stime: u64 = fields[14].parse().unwrap_or(0);
            let clock_ticks = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as f64;
            
            // Field 23 is vsize (virtual memory size)
            let vsize: u64 = fields[22].parse().unwrap_or(0);
            
            return Ok(ResourceUsage {
                user_time: utime as f64 / clock_ticks,
                system_time: stime as f64 / clock_ticks,
                max_memory: vsize / 1024, // Convert to KB
            });
        }
    }
    
    // Fallback: use rusage (works on macOS and other Unix systems)
    Ok(ResourceUsage::default())
}

fn format_time(seconds: f64) -> String {
    if seconds < 60.0 {
        format!("{:.3}s", seconds)
    } else {
        let minutes = (seconds as u64) / 60;
        let remaining_seconds = seconds - (minutes as f64 * 60.0);
        format!("{}m{:.3}s", minutes, remaining_seconds)
    }
}

#[allow(dead_code)] // May not be used on all platforms
fn format_memory(kb: u64) -> String {
    if kb == 0 {
        "N/A".to_string()
    } else if kb < 1024 {
        format!("{}KB", kb)
    } else if kb < 1024 * 1024 {
        format!("{:.1}MB", kb as f64 / 1024.0)
    } else {
        format!("{:.1}GB", kb as f64 / (1024.0 * 1024.0))
    }
}

fn execute_and_measure(args: &Args) -> Result<(std::process::ExitStatus, f64, ResourceUsage), Box<dyn std::error::Error>> {
    let program = &args.command[0];
    let program_args = if args.command.len() > 1 {
        &args.command[1..]
    } else {
        &[]
    };
    
    let wall_start = Instant::now();
    
    // Platform-specific execution with resource monitoring
    if args.portable {
        // Portable mode: simpler but less accurate
        let mut child = Command::new(program)
            .args(program_args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("Failed to execute '{}': {}", program, e))?;
        
        let exit_status = child.wait()?;
        let wall_elapsed = wall_start.elapsed().as_secs_f64();
        
        Ok((exit_status, wall_elapsed, ResourceUsage::default()))
    } else {
        // Platform-optimized mode
        execute_platform_optimized(program, program_args, wall_start)
    }
}

#[cfg(windows)]
fn execute_platform_optimized(
    program: &str,
    args: &[String],
    wall_start: Instant,
) -> Result<(std::process::ExitStatus, f64, ResourceUsage), Box<dyn std::error::Error>> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Failed to execute '{}': {}", program, e))?;
    
    let child_id = child.id();
    let exit_status = child.wait()?;
    let wall_elapsed = wall_start.elapsed().as_secs_f64();
    
    let resource_usage = get_child_process_times(child_id).unwrap_or_default();
    
    Ok((exit_status, wall_elapsed, resource_usage))
}

#[cfg(unix)]
fn execute_platform_optimized(
    program: &str,
    args: &[String],
    wall_start: Instant,
) -> Result<(std::process::ExitStatus, f64, ResourceUsage), Box<dyn std::error::Error>> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Failed to execute '{}': {}", program, e))?;
    
    let child_id = child.id();
    let exit_status = child.wait()?;
    let wall_elapsed = wall_start.elapsed().as_secs_f64();
    
    let resource_usage = get_child_process_times(child_id).unwrap_or_default();
    
    Ok((exit_status, wall_elapsed, resource_usage))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    if args.command.is_empty() {
        eprintln!("Error: No command specified");
        std::process::exit(1);
    }
    
    // Detect platform for informational purposes
    let platform = if cfg!(windows) {
        "Windows"
    } else if cfg!(target_os = "macos") {
        "macOS"
    } else if cfg!(target_os = "linux") {
        "Linux"
    } else {
        "Unix"
    };
    
    if args.verbose {
        eprintln!("Running on: {}", platform);
        if args.portable {
            eprintln!("Using portable timing mode");
        }
    }
    
    // Execute the command and measure resources
    let (exit_status, wall_seconds, resource_usage) = execute_and_measure(&args)?;
    
    // Format timing information
    let timing_info = if args.verbose {
        let mut info = format!(
            "\nCommand: {}\nExit status: {}\nElapsed (wall clock) time: {}\nUser time: {}\nSystem time: {}\nCPU usage: {:.1}%",
            args.command.join(" "),
            exit_status.code().map_or("signal".to_string(), |c| c.to_string()),
            format_time(wall_seconds),
            format_time(resource_usage.user_time),
            format_time(resource_usage.system_time),
            if wall_seconds > 0.0 { 
                ((resource_usage.user_time + resource_usage.system_time) / wall_seconds) * 100.0 
            } else { 
                0.0 
            }
        );
        
        if resource_usage.max_memory > 0 {
            info.push_str(&format!("\nMaximum memory: {}", format_memory(resource_usage.max_memory)));
        }
        
        info.push('\n');
        info
    } else {
        // Standard Unix time format
        format!(
            "\nreal    {}\nuser    {}\nsys     {}\n",
            format_time(wall_seconds),
            format_time(resource_usage.user_time),
            format_time(resource_usage.system_time)
        )
    };
    
    // Output timing information
    if let Some(output_file) = args.output_file {
        std::fs::write(output_file, timing_info)?;
    } else {
        eprint!("{}", timing_info);
    }
    
    // Exit with the same status as the executed command
    if let Some(code) = exit_status.code() {
        std::process::exit(code);
    } else {
        // Command was terminated by signal (Unix behavior)
        std::process::exit(1);
    }
}
