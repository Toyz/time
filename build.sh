#!/bin/bash
# Cross-platform build script for Unix systems

set -e

RELEASE=""
TARGET=""
ALL=false
HELP=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --release)
            RELEASE="--release"
            BUILD_DIR="release"
            shift
            ;;
        --target)
            TARGET="$2"
            shift 2
            ;;
        --all)
            ALL=true
            shift
            ;;
        --help|-h)
            HELP=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

if [ "$HELP" = true ]; then
    echo "Build script for cross-platform time command"
    echo ""
    echo "Usage: ./build.sh [options]"
    echo ""
    echo "Options:"
    echo "  --target <target>   Specific target to build for"
    echo "  --release          Build in release mode"
    echo "  --all              Build for all common targets"
    echo "  --help, -h         Show this help message"
    echo ""
    echo "Common targets:"
    echo "  x86_64-pc-windows-msvc     (Windows 64-bit)"
    echo "  x86_64-unknown-linux-gnu   (Linux 64-bit)"
    echo "  x86_64-apple-darwin        (macOS 64-bit)"
    echo "  aarch64-apple-darwin       (macOS ARM64)"
    echo "  aarch64-pc-windows-msvc    (Windows ARM64)"
    exit 0
fi

BUILD_DIR=${BUILD_DIR:-"debug"}

COMMON_TARGETS=(
    "x86_64-pc-windows-msvc"
    "x86_64-unknown-linux-gnu"
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
    "aarch64-pc-windows-msvc"
)

build_target() {
    local target_name=$1
    
    echo "🔨 Building for target: $target_name"
    
    # Add target if not already installed
    rustup target add "$target_name"
    
    # Build
    cargo build --target "$target_name" $RELEASE
    
    if [[ "$target_name" == *"windows"* ]]; then
        exe_ext=".exe"
    else
        exe_ext=""
    fi
    
    binary_path="target/$target_name/$BUILD_DIR/time$exe_ext"
    
    if [ -f "$binary_path" ]; then
        echo "✅ Successfully built: $binary_path"
        
        # Copy to a more convenient location
        output_dir="binaries/$target_name"
        mkdir -p "$output_dir"
        cp "$binary_path" "$output_dir/time$exe_ext"
        echo "📁 Copied to: $output_dir/time$exe_ext"
    else
        echo "❌ Build failed for $target_name"
    fi
}

if [ "$ALL" = true ]; then
    echo "🚀 Building for all common targets..."
    for target in "${COMMON_TARGETS[@]}"; do
        build_target "$target"
        echo ""
    done
elif [ -n "$TARGET" ]; then
    build_target "$TARGET"
else
    # Build for current platform
    echo "🔨 Building for current platform..."
    cargo build $RELEASE
    
    if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" ]]; then
        exe_ext=".exe"
    else
        exe_ext=""
    fi
    
    binary_path="target/$BUILD_DIR/time$exe_ext"
    
    if [ -f "$binary_path" ]; then
        echo "✅ Successfully built: $binary_path"
    fi
fi

echo ""
echo "🎉 Build complete!"
