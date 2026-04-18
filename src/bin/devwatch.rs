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

fn print_table(records: &[DeviceRecord]) {
    println!(
        "{:<24} {:<10} {:<12} {:<16} {:<14} {:<40} PROCESSES",
        "DEVICE", "KIND", "SUBSYSTEM", "DRIVER", "DEVNUM", "SYSFS"
    );
    println!("{}", "-".repeat(150));

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

        let proc_list = record
            .usage
            .processes
            .iter()
            .map(|p| format!("{}({})", p.name, p.pid))
            .collect::<Vec<_>>()
            .join(", ");

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
}

fn print_json(records: &[DeviceRecord]) -> io::Result<()> {
    let json = serde_json::to_string_pretty(records)
        .map_err(|e| io::Error::other(e.to_string()))?;
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

    if cli.json {
        print_json(&records)?;
    } else {
        print_table(&records);
    }

    Ok(())
}
