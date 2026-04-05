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
    #[arg(short, long, value_name = "SECONDS")]
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

struct ZswapStats {
    stored_pages: u64,
    stored_incompressible_pages: u64,
    pool_total_size: u64,
    written_back_pages: u64,
    decompress_fail: u64,
    reject_compress_poor: u64,
    reject_compress_fail: u64,
    reject_kmemcache_fail: u64,
    reject_alloc_fail: u64,
    reject_reclaim_fail: u64,
    pool_limit_hit: u64,
}

struct ZswapParams {
    enabled: String,
    shrinker_enabled: String,
    max_pool_percent: String,
    compressor: String,
    accept_threshold_percent: String,
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
        stored_incompressible_pages: read_u64(base.join("stored_incompressible_pages").as_path())?,
        pool_total_size: read_u64(base.join("pool_total_size").as_path())?,
        written_back_pages: read_u64(base.join("written_back_pages").as_path())?,
        decompress_fail: read_u64(base.join("decompress_fail").as_path())?,
        reject_compress_poor: read_u64(base.join("reject_compress_poor").as_path())?,
        reject_compress_fail: read_u64(base.join("reject_compress_fail").as_path())?,
        reject_kmemcache_fail: read_u64(base.join("reject_kmemcache_fail").as_path())?,
        reject_alloc_fail: read_u64(base.join("reject_alloc_fail").as_path())?,
        reject_reclaim_fail: read_u64(base.join("reject_reclaim_fail").as_path())?,
        pool_limit_hit: read_u64(base.join("pool_limit_hit").as_path())?,
    })
}

fn read_zswap_params() -> Option<ZswapParams> {
    let base = Path::new("/sys/module/zswap/parameters");

    Some(ZswapParams {
        enabled: read_string(base.join("enabled").as_path())?,
        shrinker_enabled: read_string(base.join("shrinker_enabled").as_path())?,
        max_pool_percent: read_string(base.join("max_pool_percent").as_path())?,
        compressor: read_string(base.join("compressor").as_path())?,
        accept_threshold_percent: read_string(base.join("accept_threshold_percent").as_path())?,
    })
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
    format_size(pages * bytes_per_page, unit, si)
}

fn get_page_size() -> u64 {
    unsafe { libc::sysconf(libc::_SC_PAGESIZE) as u64 }
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

    fn separator(mut self) -> Self {
        self.rows.push(Row::Separator);
        self
    }

    fn print(&self) {
        // Calculate column widths (only for data rows)
        let data_rows: Vec<_> = self.rows.iter().filter_map(|r| {
            if let Row::Data(l, v) = r { Some((l, v)) } else { None }
        }).collect();

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
        println!("╠{}╣", border);

        // Rows
        let mut data_idx = 0;
        for row in self.rows.iter() {
            match row {
                Row::Data(label, value) => {
                    let space_between = total_width - label.len() - value.len();
                    println!("║{}{:>space$}{}║", label, "", value, space = space_between);
                    // Add separator after each data row except the last
                    if data_idx < data_rows.len() - 1 {
                        println!("╟{}╢", separator_line);
                    }
                    data_idx += 1;
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
    let uncompressed_size = stats.stored_pages * bytes_per_page;
    let compressed_size = stats.pool_total_size;
    let space_savings = uncompressed_size.saturating_sub(compressed_size);

    let uncompressed_str = format_size(uncompressed_size, unit, si);
    let compressed_str = format_size(compressed_size, unit, si);
    let savings_str = format_size(space_savings, unit, si);
    let written_back_str = format_pages(stats.written_back_pages, bytes_per_page, unit, si);
    let incompressible_str = format_pages(stats.stored_incompressible_pages, bytes_per_page, unit, si);

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
        .data_row("Incompressible Pages", incompressible_str)
        .data_row("Written Back to Swap", written_back_str)
}

fn print_stats(stats: &ZswapStats, unit: &Unit, si: bool) {
    let mut table = build_summary_table(stats, unit, si, "ZSWAP STATISTICS");

    table = table
        .data_row("Stored Pages", stats.stored_pages.to_string())
        .data_row("Pool Limit Hits", stats.pool_limit_hit.to_string())
        .data_row("Decompress Failures", stats.decompress_fail.to_string())
        .data_row("Reject: Compress Poor", stats.reject_compress_poor.to_string())
        .data_row("Reject: Compress Fail", stats.reject_compress_fail.to_string())
        .data_row("Reject: Kmemcache Fail", stats.reject_kmemcache_fail.to_string())
        .data_row("Reject: Alloc Fail", stats.reject_alloc_fail.to_string())
        .data_row("Reject: Reclaim Fail", stats.reject_reclaim_fail.to_string());

    table.print();
}

fn print_summary(stats: &ZswapStats, unit: &Unit, si: bool) {
    build_summary_table(stats, unit, si, "ZSWAP SUMMARY").print();
}

fn print_params(params: &ZswapParams) {
    let table = Table::new("ZSWAP PARAMETERS")
        .data_row("Enabled", params.enabled.clone())
        .data_row("Shrinker", params.shrinker_enabled.clone())
        .data_row("Max Pool Percent", format!("{}%", params.max_pool_percent))
        .data_row("Compressor", params.compressor.clone())
        .data_row("Accept Threshold", format!("{}%", params.accept_threshold_percent));

    table.print();
}

fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
}

fn main() {
    let args = Args::parse();

    loop {
        let stats = match read_zswap_stats() {
            Some(s) => s,
            None => {
                eprintln!("Error: Cannot read zswap statistics. Are you root?");
                eprintln!("Try running with: sudo zswapmon");
                std::process::exit(1);
            }
        };

        if args.monitor.is_some() {
            clear_screen();
        }

        if args.parameters {
            if let Some(params) = read_zswap_params() {
                print_params(&params);
                std::process::exit(0);
            } else {
                eprintln!("Error: Could not read zswap parameters");
                std::process::exit(1);
            }
        }

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
