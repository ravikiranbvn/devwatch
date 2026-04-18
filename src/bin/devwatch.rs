use std::collections::BTreeMap;
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

    /// Show all discoverable /dev device nodes, not just currently opened ones
    #[arg(long)]
    all_devices: bool,
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

fn print_no_results() {
    println!("No matching device records found.");
}

fn subsystem_name(record: &DeviceRecord) -> &str {
    record.sysfs.subsystem.as_deref().unwrap_or("unknown")
}

fn truncate_middle(s: &str, max_width: usize) -> String {
    if s.len() <= max_width {
        return s.to_string();
    }

    if max_width <= 3 {
        return "...".to_string();
    }

    let front = (max_width - 3) / 2;
    let back = max_width - 3 - front;

    let start = s.chars().take(front).collect::<String>();
    let end = s
        .chars()
        .rev()
        .take(back)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();

    format!("{start}...{end}")
}

fn print_record_rows(record: &DeviceRecord) {
    let kind = &record.sysfs.kind;
    let driver = record.sysfs.driver.as_deref().unwrap_or("unknown");
    let devnum = record.sysfs.dev_numbers.as_deref().unwrap_or("N/A");
    let sysfs = record
        .sysfs
        .sysfs_path
        .as_ref()
        .map(|p| truncate_middle(&p.display().to_string(), 48))
        .unwrap_or_else(|| "N/A".to_string());

    if record.usage.processes.is_empty() {
        println!(
            "{:<24} {:<28} {:<10} {:<20} {:<14} {}",
            record.usage.device_path.display(),
            "-",
            kind,
            driver,
            devnum,
            sysfs
        );
        return;
    }

    for (idx, process) in record.usage.processes.iter().enumerate() {
        let proc_str = format!("{}({})", process.name, process.pid);

        if idx == 0 {
            println!(
                "{:<24} {:<28} {:<10} {:<20} {:<14} {}",
                record.usage.device_path.display(),
                proc_str,
                kind,
                driver,
                devnum,
                sysfs
            );
        } else {
            println!("{:<24} {:<28}", "", proc_str);
        }
    }
}

fn print_table(records: &[DeviceRecord], no_headers: bool) {
    if records.is_empty() {
        print_no_results();
        return;
    }

    let mut groups: BTreeMap<String, Vec<&DeviceRecord>> = BTreeMap::new();

    for record in records {
        groups
            .entry(subsystem_name(record).to_string())
            .or_default()
            .push(record);
    }

    let mut first_group = true;

    for (subsystem, group_records) in groups {
        if !first_group {
            println!();
        }
        first_group = false;

        println!("== {} ({}) ==", subsystem, group_records.len());

        if !no_headers {
            println!(
                "{:<24} {:<28} {:<10} {:<20} {:<14} SYSFS",
                "DEVICE", "PROCESSES", "KIND", "DRIVER", "DEVNUM"
            );
            println!("{}", "-".repeat(150));
        }

        for record in group_records {
            print_record_rows(record);
        }
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
    let usages = if cli.all_devices {
        dev_layer::collect_all_devices_with_usage(&processes)
    } else {
        dev_layer::collect_device_usage(&processes)
    };

    let mut records = sysfs_layer::enrich_devices_with_sysfs(&usages);

    records.retain(|record| matches_filter(record, &cli));
    records.sort_by(|a, b| a.usage.device_path.cmp(&b.usage.device_path));

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
