# Cross-Platform Unix-like Time Command

A fast, accurate, and cross-platform implementation of the Unix `time` command that works seamlessly on Windows, Linux, and macOS.

## Features

✅ **Full cross-platform support**: Windows, Linux, macOS, and other Unix-like systems  
✅ **Accurate resource measurement**: Platform-optimized timing and memory usage  
✅ **Unix-compatible interface**: Drop-in replacement for standard `time` command  
✅ **Multiple output formats**: Standard and verbose modes  
✅ **Portable mode**: Fallback for maximum compatibility  
✅ **Exit code preservation**: Returns the same exit code as the executed command  
✅ **Memory usage tracking**: Shows peak memory consumption (Linux)  

## Installation

### Pre-built Binaries

Download the appropriate binary for your platform from the `binaries/` directory after building.

### Build from Source

#### Prerequisites
- [Rust](https://rustup.rs/) (1.70 or later)
- Platform-specific dependencies are handled automatically

#### Quick Build (Current Platform)
```bash
# Windows (PowerShell)
./build.ps1 -Release

# Linux/macOS/WSL
./build.sh --release
```

#### Cross-Platform Build
```bash
# Windows (PowerShell) - Build for all platforms
./build.ps1 -All -Release

# Linux/macOS/WSL - Build for all platforms  
./build.sh --all --release

# Build for specific target
./build.ps1 -Target x86_64-unknown-linux-gnu -Release
./build.sh --target x86_64-apple-darwin --release
```

## Usage

### Basic Usage
```bash
# Time any command
time your-command arg1 arg2

# Examples
time ls -la
time ping -c 3 google.com
time python script.py
time cargo build --release
```

### Advanced Usage
```bash
# Verbose output with detailed information
time -v your-command

# Save timing info to file
time -o timing.txt your-command

# Portable mode (less accurate but more compatible)
time -p your-command

# Get help
time --help
```

## Output Formats

### Standard Format (Unix-compatible)
```
real    1.234s
user    0.567s
sys     0.123s
```

### Verbose Format (`-v` flag)
```
Running on: Linux
Command: sleep 2
Exit status: 0
Elapsed (wall clock) time: 2.003s
User time: 0.001s  
System time: 0.002s
CPU usage: 0.1%
Maximum memory: 2.1MB
```

## Platform-Specific Features

### Windows
- Uses Win32 APIs for accurate process timing
- Supports both cmd and PowerShell commands
- Works with Windows executables and scripts

### Linux  
- Reads `/proc/[pid]/stat` for detailed resource usage
- Shows peak memory consumption
- Compatible with all standard Unix tools

### macOS
- Uses BSD-style process APIs
- Full compatibility with macOS system tools
- Supports both Intel and Apple Silicon

## Cross-Platform Compatibility

The tool automatically detects the platform and uses the most accurate timing method available:

- **Windows**: Win32 `GetProcessTimes()` API
- **Linux**: `/proc` filesystem + `rusage`
- **macOS**: BSD process APIs + `rusage`
- **Other Unix**: POSIX-compatible fallback

Use `--portable` mode if you encounter compatibility issues on exotic platforms.

## Building for Multiple Platforms

This project supports cross-compilation for multiple targets:

### Supported Targets
- `x86_64-pc-windows-msvc` (Windows 64-bit)  
- `x86_64-unknown-linux-gnu` (Linux 64-bit)
- `x86_64-apple-darwin` (macOS Intel)
- `aarch64-apple-darwin` (macOS Apple Silicon)
- `aarch64-pc-windows-msvc` (Windows ARM64)

### WSL Development
The tool builds and runs perfectly in WSL (Windows Subsystem for Linux):

```bash
# In WSL - build Linux binary
./build.sh --release

# In WSL - cross-compile for Windows
./build.sh --target x86_64-pc-windows-msvc --release
```

## Why This Tool?

### vs PowerShell's `Measure-Command`
- ✅ Much simpler syntax: `time cmd` vs `Measure-Command { cmd }`
- ✅ Shows user/system time breakdown (not just wall time)
- ✅ Preserves command output and exit codes
- ✅ Cross-platform consistency
- ✅ Handles complex command lines better

### vs Windows built-in tools
- ✅ Windows doesn't have a built-in `time` command
- ✅ Provides Unix-style timing that developers expect
- ✅ Much more accurate than batch file solutions

### vs Linux/macOS `time`
- ✅ Consistent behavior across all platforms
- ✅ Enhanced verbose output with memory usage
- ✅ Modern, clean implementation in Rust

## Examples

```bash
# Time a build process
time cargo build --release

# Time a network operation  
time curl -s https://api.github.com/users/octocat

# Time a script with verbose output
time -v python data_processing.py

# Time a long-running process and save results
time -o build_timing.txt make -j8

# Compare performance across platforms
time ./benchmark_app  # Same command, different platforms
```

## License

MIT License - see [LICENSE](LICENSE) file for details.
