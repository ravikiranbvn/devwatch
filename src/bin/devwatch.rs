use std::io;

use devwatch::dev_layer;
use devwatch::model::DeviceRecord;
use devwatch::procfs_layer;
use devwatch::sysfs_layer;

fn print_records(records: &[DeviceRecord]) {
    println!(
        "{:<24} {:<10} {:<12} {:<16} {:<14} {:<40} {}",
        "DEVICE", "KIND", "SUBSYSTEM", "DRIVER", "DEVNUM", "SYSFS", "PROCESSES"
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

fn main() -> io::Result<()> {
    let processes = procfs_layer::list_processes();
    let usages = dev_layer::collect_device_usage(&processes);
    let mut records = sysfs_layer::enrich_devices_with_sysfs(&usages);

    records.sort_by(|a, b| a.usage.device_path.cmp(&b.usage.device_path));
    print_records(&records);

    Ok(())
}