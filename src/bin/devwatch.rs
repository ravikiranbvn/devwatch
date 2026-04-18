use std::io;

use clap::Parser;
use devwatch::dev_layer;
use devwatch::model::DeviceRecord;
use devwatch::procfs_layer;
use devwatch::sysfs_layer;

#[derive(Debug, Parser)]
#[command(name = "devwatch")]
#[command(about = "Linux device observability tool")]
struct Cli {
    /// Output in JSON format
    #[arg(long)]
    json: bool,

    /// Filter by device path substring
    #[arg(long)]
    device: Option<String>,

    /// Filter by subsystem
    #[arg(long)]
    subsystem: Option<String>,

    /// Filter by driver
    #[arg(long)]
    driver: Option<String>,

    /// Do not print table headers
    #[arg(long)]
    no_headers: bool,

    /// Print only record count
    #[arg(long)]
    count_only: bool,
}

fn matches_filter(record: &DeviceRecord, cli: &Cli) -> bool {
    if let Some(device_filter) = &cli.device {
        let device = record.usage.device_path.to_string_lossy();
        if !device.contains(device_filter) {
            return false;
        }
    }

    if let Some(subsystem_filter) = &cli.subsystem {
        let subsystem = record.sysfs.subsystem.as_deref().unwrap_or("");
        if subsystem != subsystem_filter {
            return false;
        }
    }

    if let Some(driver_filter) = &cli.driver {
        let driver = record.sysfs.driver.as_deref().unwrap_or("");
        if driver != driver_filter {
            return false;
        }
    }

    true
}

fn format_process_list(record: &DeviceRecord, max_width: usize) -> String {
    let full = record
        .usage
        .processes
        .iter()
        .map(|p| format!("{}({})", p.name, p.pid))
        .collect::<Vec<_>>()
        .join(", ");

    if full.len() <= max_width {
        return full;
    }

    if max_width <= 3 {
        return "...".to_string();
    }

    let mut truncated = String::new();

    for part in full.split(", ") {
        let candidate = if truncated.is_empty() {
            part.to_string()
        } else {
            format!("{truncated}, {part}")
        };

        if candidate.len() + 3 > max_width {
            break;
        }

        truncated = candidate;
    }

    if truncated.is_empty() {
        let mut s = full.chars().take(max_width - 3).collect::<String>();
        s.push_str("...");
        return s;
    }

    truncated.push_str("...");
    truncated
}

fn print_no_results() {
    println!("No matching device records found.");
}

fn print_table(records: &[DeviceRecord], no_headers: bool) {
    if records.is_empty() {
        print_no_results();
        return;
    }

    if !no_headers {
        println!(
            "{:<24} {:<10} {:<12} {:<16} {:<14} {:<40} PROCESSES",
            "DEVICE", "KIND", "SUBSYSTEM", "DRIVER", "DEVNUM", "SYSFS"
        );
        println!("{}", "-".repeat(150));
    }

    for record in records {
        let kind = &record.sysfs.kind;
        let subsystem = record.sysfs.subsystem.as_deref().unwrap_or("unknown");
        let driver = record.sysfs.driver.as_deref().unwrap_or("unknown");
        let devnum = record.sysfs.dev_numbers.as_deref().unwrap_or("N/A");
        let sysfs = record
            .sysfs
            .sysfs_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "N/A".to_string());

        let proc_list = format_process_list(record, 48);

        println!(
            "{:<24} {:<10} {:<12} {:<16} {:<14} {:<40} {}",
            record.usage.device_path.display(),
            kind,
            subsystem,
            driver,
            devnum,
            sysfs,
            proc_list
        );
    }

    if !no_headers {
        println!();
        println!("{} record(s)", records.len());
    }
}

fn print_json(records: &[DeviceRecord]) -> io::Result<()> {
    let json =
        serde_json::to_string_pretty(records).map_err(|e| io::Error::other(e.to_string()))?;
    println!("{json}");
    Ok(())
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let processes = procfs_layer::list_processes();
    let usages = dev_layer::collect_device_usage(&processes);
    let mut records = sysfs_layer::enrich_devices_with_sysfs(&usages);

    records.retain(|record| matches_filter(record, &cli));
    records.sort_by(|a, b| a.usage.device_path.cmp(&b.usage.device_path));

    // Count-only mode (highest priority)
    if cli.count_only {
        println!("{}", records.len());
        return Ok(());
    }

    if cli.json {
        print_json(&records)?;
    } else {
        print_table(&records, cli.no_headers);
    }

    Ok(())
}
