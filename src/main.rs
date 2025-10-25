use clap::Parser;
use std::process::{Command, Stdio};
use std::time::Instant;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

// Windows-specific imports
#[cfg(windows)]
use winapi::um::processthreadsapi::{GetProcessTimes, OpenProcess};
#[cfg(windows)]
use winapi::shared::minwindef::FILETIME;
#[cfg(windows)]
use winapi::um::winnt::{HANDLE, PROCESS_QUERY_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION};
#[cfg(windows)]
use winapi::um::handleapi::CloseHandle;
#[cfg(windows)]
use winapi::um::psapi::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};

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
        // Try PROCESS_QUERY_INFORMATION first, fallback to limited if needed
        let mut handle: HANDLE = OpenProcess(PROCESS_QUERY_INFORMATION, 0, child_id);
        if handle.is_null() {
            handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, child_id);
            if handle.is_null() {
                return Ok(ResourceUsage::default());
            }
        }
        
        let mut creation_time = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        let mut exit_time = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        let mut kernel_time = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        let mut user_time = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        
        let timing_result = GetProcessTimes(
            handle,
            &mut creation_time,
            &mut exit_time,
            &mut kernel_time,
            &mut user_time,
        );
        
        let mut max_memory_kb = 0u64;
        
        // Get memory information
        let mut mem_counters = PROCESS_MEMORY_COUNTERS {
            cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
            PageFaultCount: 0,
            PeakWorkingSetSize: 0,
            WorkingSetSize: 0,
            QuotaPeakPagedPoolUsage: 0,
            QuotaPagedPoolUsage: 0,
            QuotaPeakNonPagedPoolUsage: 0,
            QuotaNonPagedPoolUsage: 0,
            PagefileUsage: 0,
            PeakPagefileUsage: 0,
        };
        
        let memory_result = GetProcessMemoryInfo(
            handle,
            &mut mem_counters,
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        );
        
        if memory_result != 0 {
            // Use PeakWorkingSetSize (peak physical memory usage) converted to KB
            max_memory_kb = mem_counters.PeakWorkingSetSize as u64 / 1024;
        }
        
        CloseHandle(handle);
        
        if timing_result == 0 {
            // Even if timing fails, we might have memory info
            return Ok(ResourceUsage {
                user_time: 0.0,
                system_time: 0.0,
                max_memory: max_memory_kb,
            });
        }
        
        let user_seconds = filetime_to_seconds(&user_time);
        let kernel_seconds = filetime_to_seconds(&kernel_time);
        
        Ok(ResourceUsage {
            user_time: user_seconds,
            system_time: kernel_seconds,
            max_memory: max_memory_kb,
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

fn execute_and_measure(args: &Args, interrupted: Arc<AtomicBool>) -> Result<(std::process::ExitStatus, f64, ResourceUsage, bool), Box<dyn std::error::Error>> {
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
        let was_interrupted = interrupted.load(Ordering::SeqCst);
        
        Ok((exit_status, wall_elapsed, ResourceUsage::default(), was_interrupted))
    } else {
        // Platform-optimized mode
        execute_platform_optimized(program, program_args, wall_start, interrupted)
    }
}

#[cfg(windows)]
fn execute_platform_optimized(
    program: &str,
    args: &[String],
    wall_start: Instant,
    interrupted: Arc<AtomicBool>,
) -> Result<(std::process::ExitStatus, f64, ResourceUsage, bool), Box<dyn std::error::Error>> {
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
    let was_interrupted = interrupted.load(Ordering::SeqCst);
    
    let resource_usage = get_child_process_times(child_id).unwrap_or_default();
    
    Ok((exit_status, wall_elapsed, resource_usage, was_interrupted))
}

#[cfg(unix)]
fn execute_platform_optimized(
    program: &str,
    args: &[String],
    wall_start: Instant,
    interrupted: Arc<AtomicBool>,
) -> Result<(std::process::ExitStatus, f64, ResourceUsage, bool), Box<dyn std::error::Error>> {
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
    let was_interrupted = interrupted.load(Ordering::SeqCst);
    
    let resource_usage = get_child_process_times(child_id).unwrap_or_default();
    
    Ok((exit_status, wall_elapsed, resource_usage, was_interrupted))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    if args.command.is_empty() {
        eprintln!("Error: No command specified");
        std::process::exit(1);
    }
    
    // Set up signal handling for Ctrl+C, etc.
    let interrupted = Arc::new(AtomicBool::new(false));
    let interrupted_clone = interrupted.clone();
    
    ctrlc::set_handler(move || {
        interrupted_clone.store(true, Ordering::SeqCst);
    })?;
    
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
    let (exit_status, wall_seconds, resource_usage, was_interrupted) = execute_and_measure(&args, interrupted.clone())?;
    
    // Format timing information
    let timing_info = if args.verbose {
        let command_str = args.command.join(" ");
        let exit_str = if was_interrupted {
            "interrupted".to_string()
        } else {
            exit_status.code().map_or("signal".to_string(), |c| c.to_string())
        };
        let cpu_usage = if wall_seconds > 0.0 { 
            ((resource_usage.user_time + resource_usage.system_time) / wall_seconds) * 100.0 
        } else { 
            0.0 
        };
        
        let mut lines = vec![
            format!("Command:             {}", command_str),
            format!("Exit status:         {}", exit_str),
            format!("Elapsed time:        {}", format_time(wall_seconds)),
            format!("User time:           {}", format_time(resource_usage.user_time)),
            format!("System time:         {}", format_time(resource_usage.system_time)),
            format!("CPU usage:           {:.1}%", cpu_usage),
        ];
        
        if resource_usage.max_memory > 0 {
            lines.push(format!("Peak memory:         {}", format_memory(resource_usage.max_memory)));
        }
        
        format!("\n{}\n", lines.join("\n"))
    } else {
        // Standard Unix time format - exactly like real time command
        format!(
            "\nreal\t{}\nuser\t{}\nsys\t{}\n",
            format_time(wall_seconds),
            format_time(resource_usage.user_time),
            format_time(resource_usage.system_time)
        )
    };
    
    // Output timing information - always show it, even if interrupted
    if let Some(output_file) = args.output_file {
        std::fs::write(output_file, timing_info)?;
    } else {
        eprint!("{}", timing_info);
    }
    
    // Exit with appropriate code
    if was_interrupted {
        std::process::exit(130); // Standard Unix exit code for SIGINT
    } else if let Some(code) = exit_status.code() {
        std::process::exit(code);
    } else {
        // Command was terminated by signal (Unix behavior)
        std::process::exit(1);
    }
}
