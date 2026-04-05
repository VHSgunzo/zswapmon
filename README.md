# zswapmon

A command-line utility for monitoring Linux zswap compressed swap cache statistics and parameters.

## Overview

`zswapmon` provides real-time monitoring of zswap, a Linux kernel feature that compresses pages in RAM before swapping them to disk. This tool helps you understand memory compression efficiency, track swap activity, and monitor kernel parameters.

## Features

- **Summary View**: Quick overview of compression ratios, space savings, and page statistics
- **Detailed Statistics**: Extended view with rejection counts and failure metrics
- **Parameter Inspection**: View current zswap kernel configuration
- **Continuous Monitoring**: Real-time updates with customizable intervals
- **Flexible Units**: Automatic or manual size unit selection (B, KiB, MiB, GiB)
- **SI/Binary Support**: Toggle between decimal (KB/MB/GB) and binary (KiB/MiB/GiB) units

## Requirements

- **Root privileges**: Reading zswap statistics requires access to `/sys/kernel/debug/zswap/`
- **Linux kernel**: zswap support (enabled by default in most modern kernels)
- **Kernel 3.11+**: When zswap was introduced

## Building from Source

### Prerequisites

- Rust nightly toolchain
- x86_64-unknown-linux-musl target installed

```bash
rustup toolchain install nightly
rustup target add x86_64-unknown-linux-musl --toolchain nightly
```

### Build Configuration

The project uses several optimizations for minimal, portable binaries:

- **Toolchain**: Rust nightly (see [`rust-toolchain.toml`](rust-toolchain.toml))
- **Target**: `x86_64-unknown-linux-musl` for static linking (see [`.cargo/config.toml`](.cargo/config.toml))
- **Release profile optimizations**:
  - Link-time optimization (LTO)
  - Maximum size optimization (`opt-level = "z"`)
  - Symbol stripping
  - Single codegen unit
  - Immediate panic abort

## Installation

### From Source

```bash
git clone <repository-url>
cd zswapmon
cargo build --release
sudo cp target/x86_64-unknown-linux-musl/release/zswapmon /usr/local/bin/
```

## Usage

### Quick Start

```bash
# Basic summary (default view)
sudo zswapmon

# Detailed statistics
sudo zswapmon --stats

# View zswap parameters
sudo zswapmon --parameters

# Continuous monitoring (refresh every 5 seconds)
sudo zswapmon --monitor 5
```

### Command-Line Options

```
OPTIONS:
    -u, --unit <UNIT>      Size display unit: auto, b, k, m, g (default: auto)
        --si               Use SI decimal units (KB=1000) instead of binary (KiB=1024)
    -s, --stats            Display detailed statistics with rejection counts
    -p, --parameters       Display zswap kernel parameters
    -m, --monitor <SEC>    Continuous monitoring mode, refresh every N seconds
    -h, --help             Show help information
```

### Unit Options

- `auto` - Automatically scale to best unit (default)
- `b` - Bytes
- `k` - Kibibytes (KiB) or Kilobytes (KB) with --si
- `m` - Mebibytes (MiB) or Megabytes (MB) with --si
- `g` - Gibibytes (GiB) or Gigabytes (GB) with --si

## Output Examples

### Summary View

```
╔════════════════════════════════════════╗
║             ZSWAP SUMMARY              ║
╠════════════════════════════════════════╣
║Uncompressed Data                20.2GiB║
╟────────────────────────────────────────╢
║Compressed Data                  14.6GiB║
╟────────────────────────────────────────╢
║Space Savings                     5.6GiB║
╟────────────────────────────────────────╢
╠════════════════════════════════════════╣
║Compression Ratio    1.38x (27.6% saved)║
╟────────────────────────────────────────╢
╠════════════════════════════════════════╣
║Incompressible Pages              7.7GiB║
╟────────────────────────────────────────╢
║Written Back to Swap             12.5GiB║
╚════════════════════════════════════════╝
```

### Detailed Statistics

```bash
sudo zswapmon --stats
```

Shows additional metrics:
- Stored pages count
- Pool limit hits
- Decompression failures
- Rejection reasons (compress poor/fail, kmemcache fail, alloc fail, reclaim fail)

### Parameters View

```bash
sudo zswapmon --parameters
```

Displays:
- Enabled status
- Shrinker configuration
- Max pool percentage
- Active compressor algorithm
- Accept threshold percentage

## Understanding zswap

### What is zswap?

zswap is a Linux kernel feature that provides a compressed write-back cache for swapped pages. Instead of immediately writing swapped pages to disk, zswap:

1. Compresses pages in RAM
2. Stores them in a memory pool
3. Only writes to disk when the pool is full or pages are incompressible

This improves performance by reducing disk I/O while using less RAM than uncompressed swap.

### Key Metrics

- **Compression Ratio**: How effectively data is compressed (higher is better)
- **Space Savings**: Amount of RAM saved through compression
- **Incompressible Pages**: Pages that couldn't be compressed (already compressed random data)
- **Written Back Pages**: Pages flushed to disk when pool reached capacity
- **Reject Counts**: Pages rejected due to various failure conditions

### Configuration Parameters

- `enabled`: Whether zswap is active (Y/N)
- `shrinker_enabled`: Enable memory pressure shrinking (Y/N)
- `max_pool_percent`: Maximum RAM percentage for compressed pool (default: 25%)
- `compressor`: Compression algorithm (lzo, lz4, zstd, deflate)
- `accept_threshold_percent`: Pool utilization threshold for accepting new pages

## Troubleshooting

### "Cannot read zswap statistics"

Ensure you're running with root privileges:
```bash
sudo zswapmon
```

### zswap not enabled

Check if zswap is active:
```bash
cat /sys/module/zswap/parameters/enabled
```

Enable it if needed:
```bash
echo Y > /sys/module/zswap/parameters/enabled
```

### No statistics available

Verify debug filesystem is mounted:
```bash
mount | grep debugfs
# If not mounted:
mount -t debugfs debugfs /sys/kernel/debug
```

## License

MIT License - see LICENSE file for details.

## Author

VHSgunzo <vhsgunzo@gmail.com>

## Contributing

Contributions welcome! Please feel free to submit pull requests or open issues for bugs and feature requests.
