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
  (`--parameters` reads `/sys/module/zswap/parameters/` and does not need debugfs)
- **debugfs** mounted at `/sys/kernel/debug`
- **Linux kernel 5.15 or newer** with zswap support

### Kernel compatibility

The set of zswap counters and parameters differs between kernel versions.
Only `stored_pages` and `pool_total_size` are required; every other row is
shown only if the running kernel provides it, so output is kernel version
dependent.

| debugfs counter / module parameter | Kernels |
|---|---|
| `stored_pages`, `pool_total_size` (required) | all |
| `written_back_pages`, `pool_limit_hit`, `reject_alloc_fail`, `reject_kmemcache_fail`, `reject_reclaim_fail`, `reject_compress_poor` | all |
| `duplicate_entry` | 5.15 – 6.8 |
| `same_filled_pages` | 5.15 – 6.11 |
| `reject_compress_fail` | 6.7+ |
| `decompress_fail` | 6.15+ |
| `stored_incompressible_pages` | 6.18+ |
| param `enabled`, `compressor`, `max_pool_percent`, `accept_threshold_percent` | all |
| param `zpool` | 5.15 – 6.17 |
| param `same_filled_pages_enabled` | 5.15 – 6.7 |
| param `non_same_filled_pages_enabled` | 5.18 – 6.7 |
| param `shrinker_enabled` | 6.8+ |

On kernels up to 6.11, same-filled pages (e.g. all-zero pages) are counted in
`stored_pages` but take no space in the compressed pool, so the compression
ratio there is higher than the actual compressor ratio.

## Building from Source

### Prerequisites

- Rust stable toolchain, 1.85 or newer (`rust-toolchain.toml` selects `stable`)
- x86_64-unknown-linux-musl target installed

```bash
rustup toolchain install stable
rustup target add x86_64-unknown-linux-musl --toolchain stable
```

### Build Configuration

The project uses several optimizations for minimal, portable binaries:

- **Toolchain**: Rust stable (see [`rust-toolchain.toml`](rust-toolchain.toml))
- **Target**: `x86_64-unknown-linux-musl` for static linking (see [`.cargo/config.toml`](.cargo/config.toml))
- **Release profile optimizations**:
  - Link-time optimization (LTO)
  - Maximum size optimization (`opt-level = "z"`)
  - Symbol stripping
  - Single codegen unit
  - Panic abort (no unwinding)

## Installation

### From Source

```bash
git clone https://github.com/VHSgunzo/zswapmon.git
cd zswapmon
cargo build --release
sudo cp target/x86_64-unknown-linux-musl/release/zswapmon /usr/local/bin/
```

### From [releases](https://github.com/VHSgunzo/zswapmon/releases)
```
wget https://github.com/VHSgunzo/zswapmon/releases/latest/download/zswapmon
chmod +x zswapmon
sudo mv zswapmon /usr/local/bin/
```

### From [AUR](https://aur.archlinux.org/packages/zswapmon-bin)

```bash
yay -S zswapmon-bin
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
╠════════════════════════════════════════╣
║Compression Ratio    1.38x (27.6% saved)║
╠════════════════════════════════════════╣
║Incompressible Pages              7.7GiB║
╟────────────────────────────────────────╢
║Written Back to Swap             12.5GiB║
╚════════════════════════════════════════╝
```

Rows for counters the running kernel does not provide are hidden
(see [Kernel compatibility](#kernel-compatibility)). For example,
"Incompressible Pages" only appears on 6.18+, and "Same-Filled Pages"
only on kernels up to 6.11.

### Detailed Statistics

```bash
sudo zswapmon --stats
```

Shows additional metrics (where provided by the kernel):
- Stored pages count
- Same-filled pages and duplicate entries (older kernels)
- Pool limit hits
- Decompression failures
- Rejection reasons (compress poor/fail, kmemcache fail, alloc fail, reclaim fail)

### Parameters View

```bash
sudo zswapmon --parameters
```

Displays (where provided by the kernel):
- Enabled status
- Shrinker configuration
- Max pool percentage
- Active compressor algorithm
- Zpool allocator
- Accept threshold percentage
- Same-filled / non-same-filled page handling (older kernels)

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
- `zpool`: Compressed memory allocator (zbud, z3fold, zsmalloc; kernels up to 6.17)
- `accept_threshold_percent`: Pool utilization threshold for accepting new pages
- `same_filled_pages_enabled` / `non_same_filled_pages_enabled`: Which page types zswap stores (kernels up to 6.7)

## Troubleshooting

### "Error: cannot read /sys/kernel/debug/zswap/..."

The message names the file and the reason, followed by a hint:

- **Permission denied**, not root: run with `sudo zswapmon`.
- **Permission denied** as root: kernel lockdown (often enabled with Secure
  Boot) blocks debugfs; check `cat /sys/kernel/security/lockdown`.
- **No such file or directory**: debugfs is not mounted, or zswap is not
  available in this kernel (see below).

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
