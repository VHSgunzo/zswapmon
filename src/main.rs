use clap::{Parser, ValueEnum};
use std::fs;
use std::path::Path;

/// Command-line tool for monitoring zswap status and statistics
#[derive(Parser)]
#[command(name = "zswapmon")]
#[command(about = "Monitor zswap status and statistics")]
#[command(long_about = "A utility for monitoring Linux zswap compressed swap cache, providing real-time statistics and parameter inspection.")]
pub struct Args {
    /// Display size unit for all measurements: auto (automatic scaling), B (bytes), K (kibibytes), M (mebibytes), G (gibibytes)
    #[arg(short, long, value_enum, default_value = "auto", value_name = "UNIT")]
    unit: Unit,

    /// Use SI decimal units (KB=1000, MB=1000000) instead of binary units (KiB=1024, MiB=1048576)
    #[arg(long, default_value_t = false)]
    si: bool,

    /// Display detailed statistics including rejection counts, failures, and pool limit hits
    #[arg(short, long, default_value_t = false)]
    stats: bool,

    /// Display current zswap kernel parameters (enabled, compressor, thresholds, etc.)
    #[arg(short, long, default_value_t = false)]
    parameters: bool,

    /// Enable continuous monitoring mode, refreshing display every N seconds
    #[arg(short, long, value_name = "SECONDS", value_parser = clap::value_parser!(u64).range(1..))]
    monitor: Option<u64>,
}

#[derive(Clone, ValueEnum, Default)]
enum Unit {
    #[default]
    Auto,
    B,
    K,
    M,
    G,
}

// Only stored_pages and pool_total_size exist on every supported kernel (5.15+).
// Other counters are None when the running kernel does not provide them.
struct ZswapStats {
    stored_pages: u64,
    stored_incompressible_pages: Option<u64>, // 6.18+
    pool_total_size: u64,
    written_back_pages: Option<u64>,
    decompress_fail: Option<u64>, // 6.15+
    reject_compress_poor: Option<u64>,
    reject_compress_fail: Option<u64>, // 6.7+
    reject_kmemcache_fail: Option<u64>,
    reject_alloc_fail: Option<u64>,
    reject_reclaim_fail: Option<u64>,
    pool_limit_hit: Option<u64>,
    same_filled_pages: Option<u64>, // up to 6.11
    duplicate_entry: Option<u64>, // up to 6.8
}

struct ZswapParams {
    enabled: Option<String>,
    shrinker_enabled: Option<String>, // 6.8+
    max_pool_percent: Option<String>,
    compressor: Option<String>,
    accept_threshold_percent: Option<String>,
    zpool: Option<String>, // up to 6.17
    same_filled_pages_enabled: Option<String>, // up to 6.7
    non_same_filled_pages_enabled: Option<String>, // 5.18 - 6.7
}


fn read_u64(path: &Path) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn read_string(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn read_zswap_stats() -> Option<ZswapStats> {
    let base = Path::new("/sys/kernel/debug/zswap");

    Some(ZswapStats {
        stored_pages: read_u64(base.join("stored_pages").as_path())?,
        stored_incompressible_pages: read_u64(base.join("stored_incompressible_pages").as_path()),
        pool_total_size: read_u64(base.join("pool_total_size").as_path())?,
        written_back_pages: read_u64(base.join("written_back_pages").as_path()),
        decompress_fail: read_u64(base.join("decompress_fail").as_path()),
        reject_compress_poor: read_u64(base.join("reject_compress_poor").as_path()),
        reject_compress_fail: read_u64(base.join("reject_compress_fail").as_path()),
        reject_kmemcache_fail: read_u64(base.join("reject_kmemcache_fail").as_path()),
        reject_alloc_fail: read_u64(base.join("reject_alloc_fail").as_path()),
        reject_reclaim_fail: read_u64(base.join("reject_reclaim_fail").as_path()),
        pool_limit_hit: read_u64(base.join("pool_limit_hit").as_path()),
        same_filled_pages: read_u64(base.join("same_filled_pages").as_path()),
        duplicate_entry: read_u64(base.join("duplicate_entry").as_path()),
    })
}

fn read_zswap_params() -> Option<ZswapParams> {
    let base = Path::new("/sys/module/zswap/parameters");
    if !base.is_dir() {
        return None;
    }

    Some(ZswapParams {
        enabled: read_string(base.join("enabled").as_path()),
        shrinker_enabled: read_string(base.join("shrinker_enabled").as_path()),
        max_pool_percent: read_string(base.join("max_pool_percent").as_path()),
        compressor: read_string(base.join("compressor").as_path()),
        accept_threshold_percent: read_string(base.join("accept_threshold_percent").as_path()),
        zpool: read_string(base.join("zpool").as_path()),
        same_filled_pages_enabled: read_string(base.join("same_filled_pages_enabled").as_path()),
        non_same_filled_pages_enabled: read_string(base.join("non_same_filled_pages_enabled").as_path()),
    })
}

fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

fn debugfs_mounted() -> bool {
    fs::read_to_string("/proc/mounts")
        .map(|m| m.lines().any(|l| l.split_whitespace().nth(1) == Some("/sys/kernel/debug")
            && l.split_whitespace().nth(2) == Some("debugfs")))
        .unwrap_or(false)
}

/// Explain why reading the required zswap statistics failed.
fn report_stats_error() {
    let base = Path::new("/sys/kernel/debug/zswap");
    for name in ["stored_pages", "pool_total_size"] {
        let path = base.join(name);
        let err = match fs::read_to_string(&path) {
            Ok(_) => continue,
            Err(e) => e,
        };
        eprintln!("Error: cannot read {}: {}", path.display(), err);
        match err.kind() {
            std::io::ErrorKind::PermissionDenied if !is_root() => {
                eprintln!("zswap statistics are only readable by root. Try: sudo zswapmon");
            }
            std::io::ErrorKind::PermissionDenied => {
                eprintln!("Access was denied even though running as root.");
                eprintln!("Kernel lockdown may be blocking debugfs; see /sys/kernel/security/lockdown");
            }
            std::io::ErrorKind::NotFound if !debugfs_mounted() => {
                eprintln!("debugfs is not mounted. Mount it with:");
                eprintln!("  sudo mount -t debugfs debugfs /sys/kernel/debug");
            }
            std::io::ErrorKind::NotFound if !base.exists() => {
                eprintln!("{} does not exist: zswap is not available or not initialised in this kernel.", base.display());
                eprintln!("Check /sys/module/zswap/parameters/enabled");
            }
            _ => {}
        }
        return;
    }
    eprintln!("Error: Cannot read zswap statistics from {}", base.display());
}

fn format_size(bytes: u64, unit: &Unit, si: bool) -> String {
    let (unit_size, suffixes) = if si {
        let suffixes = ["B", "KB", "MB", "GB", "TB"];
        (1000.0, suffixes)
    } else {
        let suffixes = ["B", "KiB", "MiB", "GiB", "TiB"];
        (1024.0, suffixes)
    };

    match unit {
        Unit::Auto => {
            if bytes == 0 {
                return "0B".to_string();
            }
            let size = bytes as f64;
            let mut exp = 0;
            let mut value = size;
            while value >= unit_size && exp < suffixes.len() - 1 {
                value /= unit_size;
                exp += 1;
            }
            if exp == 0 {
                format!("{}{}", bytes, suffixes[0])
            } else {
                format!("{:.1}{}", value, suffixes[exp])
            }
        }
        Unit::B => format!("{}B", bytes),
        Unit::K => {
            let value = bytes as f64 / unit_size;
            format!("{:.1}{}", value, suffixes[1])
        }
        Unit::M => {
            let value = bytes as f64 / unit_size.powi(2);
            format!("{:.1}{}", value, suffixes[2])
        }
        Unit::G => {
            let value = bytes as f64 / unit_size.powi(3);
            format!("{:.1}{}", value, suffixes[3])
        }
    }
}

fn format_pages(pages: u64, bytes_per_page: u64, unit: &Unit, si: bool) -> String {
    format_size(pages.saturating_mul(bytes_per_page), unit, si)
}

fn get_page_size() -> u64 {
    let size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if size > 0 { size as u64 } else { 4096 }
}

/// Table row that can have a separator line after it
#[derive(Clone)]
enum Row {
    Data(String, String),
    Separator,
}

struct Table {
    rows: Vec<Row>,
    title: String,
}

impl Table {
    fn new(title: &str) -> Self {
        Table {
            rows: Vec::new(),
            title: title.to_string(),
        }
    }

    fn data_row(mut self, label: &str, value: String) -> Self {
        self.rows.push(Row::Data(label.to_string(), value));
        self
    }

    /// Add a row only if the value is provided by the running kernel
    fn optional_row(self, label: &str, value: Option<String>) -> Self {
        match value {
            Some(value) => self.data_row(label, value),
            None => self,
        }
    }

    fn separator(mut self) -> Self {
        self.rows.push(Row::Separator);
        self
    }

    fn print(&self) {
        // Calculate column widths (only for data rows)
        let data_rows: Vec<_> = self.rows.iter().filter_map(|r| {
            if let Row::Data(l, v) = r { Some((l, v)) } else { None }
        }).collect();

        // Drop leading, trailing and repeated separators left by hidden rows
        let mut rows: Vec<&Row> = Vec::new();
        for row in self.rows.iter() {
            if matches!(row, Row::Separator) && matches!(rows.last(), None | Some(Row::Separator)) {
                continue;
            }
            rows.push(row);
        }
        while matches!(rows.last(), Some(Row::Separator)) {
            rows.pop();
        }

        let label_width = data_rows.iter().map(|(l, _)| l.len()).max().unwrap_or(0);
        let value_width = data_rows.iter().map(|(_, v)| v.len()).max().unwrap_or(0);
        let title_width = self.title.len();
        let content_width = label_width + 1 + value_width;
        let total_width = std::cmp::max(content_width, title_width);

        let border = "═".repeat(total_width);
        let separator_line = "─".repeat(total_width);

        // Top border
        println!("╔{}╗", border);

        // Title
        let pad_left = (total_width - self.title.len()) / 2;
        let pad_right = total_width - self.title.len() - pad_left;
        println!("║{:pad_left$}{}{:pad_right$}║", "", self.title, "", pad_left = pad_left, pad_right = pad_right);

        // Separator after title
        if !rows.is_empty() {
            println!("╠{}╣", border);
        }

        // Rows
        for (i, row) in rows.iter().enumerate() {
            match row {
                Row::Data(label, value) => {
                    let space_between = total_width - label.len() - value.len();
                    println!("║{}{:>space$}{}║", label, "", value, space = space_between);
                    // Add separator between consecutive data rows
                    if matches!(rows.get(i + 1), Some(Row::Data(..))) {
                        println!("╟{}╢", separator_line);
                    }
                }
                Row::Separator => {
                    println!("╠{}╣", "═".repeat(total_width));
                }
            }
        }

        // Bottom border
        println!("╚{}╝", border);
    }
}

fn build_summary_table(stats: &ZswapStats, unit: &Unit, si: bool, title: &str) -> Table {
    let bytes_per_page = get_page_size();
    let uncompressed_size = stats.stored_pages.saturating_mul(bytes_per_page);
    let compressed_size = stats.pool_total_size;
    let space_savings = uncompressed_size.saturating_sub(compressed_size);

    let uncompressed_str = format_size(uncompressed_size, unit, si);
    let compressed_str = format_size(compressed_size, unit, si);
    let savings_str = format_size(space_savings, unit, si);
    let written_back_str = stats.written_back_pages.map(|p| format_pages(p, bytes_per_page, unit, si));
    let incompressible_str = stats.stored_incompressible_pages.map(|p| format_pages(p, bytes_per_page, unit, si));
    let same_filled_str = stats.same_filled_pages.map(|p| format_pages(p, bytes_per_page, unit, si));

    let compression_str = if stats.stored_pages > 0 && compressed_size > 0 {
        let ratio = uncompressed_size as f64 / compressed_size as f64;
        let savings_percent = (space_savings as f64 / uncompressed_size as f64) * 100.0;
        format!("{:.2}x ({:.1}% saved)", ratio, savings_percent)
    } else {
        "N/A".to_string()
    };

    Table::new(title)
        .data_row("Uncompressed Data", uncompressed_str)
        .data_row("Compressed Data", compressed_str)
        .data_row("Space Savings", savings_str)
        .separator()
        .data_row("Compression Ratio", compression_str)
        .separator()
        .optional_row("Incompressible Pages", incompressible_str)
        .optional_row("Same-Filled Pages", same_filled_str)
        .optional_row("Written Back to Swap", written_back_str)
}

fn print_stats(stats: &ZswapStats, unit: &Unit, si: bool) {
    let mut table = build_summary_table(stats, unit, si, "ZSWAP STATISTICS");
    let count = |c: Option<u64>| c.map(|c| c.to_string());

    table = table
        .separator()
        .data_row("Stored Pages", stats.stored_pages.to_string())
        .optional_row("Same-Filled Pages Count", count(stats.same_filled_pages))
        .optional_row("Duplicate Entries", count(stats.duplicate_entry))
        .optional_row("Pool Limit Hits", count(stats.pool_limit_hit))
        .optional_row("Decompress Failures", count(stats.decompress_fail))
        .optional_row("Reject: Compress Poor", count(stats.reject_compress_poor))
        .optional_row("Reject: Compress Fail", count(stats.reject_compress_fail))
        .optional_row("Reject: Kmemcache Fail", count(stats.reject_kmemcache_fail))
        .optional_row("Reject: Alloc Fail", count(stats.reject_alloc_fail))
        .optional_row("Reject: Reclaim Fail", count(stats.reject_reclaim_fail));

    table.print();
}

fn print_summary(stats: &ZswapStats, unit: &Unit, si: bool) {
    build_summary_table(stats, unit, si, "ZSWAP SUMMARY").print();
}

fn print_params(params: &ZswapParams) {
    let table = Table::new("ZSWAP PARAMETERS")
        .optional_row("Enabled", params.enabled.clone())
        .optional_row("Shrinker", params.shrinker_enabled.clone())
        .optional_row("Max Pool Percent", params.max_pool_percent.as_ref().map(|v| format!("{}%", v)))
        .optional_row("Compressor", params.compressor.clone())
        .optional_row("Zpool", params.zpool.clone())
        .optional_row("Accept Threshold", params.accept_threshold_percent.as_ref().map(|v| format!("{}%", v)))
        .optional_row("Same-Filled Pages", params.same_filled_pages_enabled.clone())
        .optional_row("Non-Same-Filled Pages", params.non_same_filled_pages_enabled.clone());

    table.print();
}

fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
}

fn main() {
    let args = Args::parse();

    loop {
        if args.monitor.is_some() {
            clear_screen();
        }

        if args.parameters {
            if let Some(params) = read_zswap_params() {
                print_params(&params);
                std::process::exit(0);
            } else {
                eprintln!("Error: Could not read zswap parameters from /sys/module/zswap/parameters");
                eprintln!("zswap is not available in this kernel.");
                std::process::exit(1);
            }
        }

        let stats = match read_zswap_stats() {
            Some(s) => s,
            None => {
                report_stats_error();
                std::process::exit(1);
            }
        };

        if args.stats {
            print_stats(&stats, &args.unit, args.si);
        } else {
            print_summary(&stats, &args.unit, args.si);
        }

        if let Some(interval) = args.monitor {
            std::thread::sleep(std::time::Duration::from_secs(interval));
        } else {
            break;
        }
    }
}
