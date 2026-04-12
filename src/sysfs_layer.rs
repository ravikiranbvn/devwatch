use std::fs;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};

use crate::model::{DeviceRecord, DeviceUsage, SysfsInfo};

fn read_dev_numbers_from_sysfs(path: &Path) -> Option<String> {
    let dev_file = path.join("dev");
    let content = fs::read_to_string(dev_file).ok()?;
    Some(content.trim().to_string())
}

fn read_subsystem_from_sysfs(path: &Path) -> Option<String> {
    let subsystem_link = path.join("subsystem");
    let target = fs::read_link(subsystem_link).ok()?;
    let name = target.file_name()?;
    Some(name.to_string_lossy().to_string())
}

fn read_driver_name(driver_link: &Path) -> Option<String> {
    let target = fs::read_link(driver_link).ok()?;
    let name = target.file_name()?;
    Some(name.to_string_lossy().to_string())
}

fn read_driver_direct(path: &Path) -> Option<String> {
    read_driver_name(&path.join("device").join("driver"))
        .or_else(|| read_driver_name(&path.join("driver")))
}

fn walk_up_for_driver(start: &Path) -> Option<String> {
    let mut current = start.to_path_buf();

    loop {
        if let Some(driver) = read_driver_direct(&current) {
            return Some(driver);
        }

        if !current.pop() {
            break;
        }
    }

    None
}

fn device_numbers_from_devnode(dev_path: &Path) -> Option<(u64, u64)> {
    let metadata = fs::metadata(dev_path).ok()?;
    let file_type = metadata.file_type();

    if !file_type.is_char_device() && !file_type.is_block_device() {
        return None;
    }

    let rdev = metadata.rdev();

    let major = ((rdev >> 8) & 0xfff) | ((rdev >> 32) & !0xfff);
    let minor = (rdev & 0xff) | ((rdev >> 12) & !0xff);

    Some((major, minor))
}

fn find_sysfs_path_by_devnums(major: u64, minor: u64) -> Option<PathBuf> {
    let class_root = Path::new("/sys/class");
    let want = format!("{major}:{minor}");

    for class_entry in fs::read_dir(class_root).ok()? {
        let class_entry = class_entry.ok()?;
        let class_path = class_entry.path();

        let devices = match fs::read_dir(&class_path) {
            Ok(v) => v,
            Err(_) => continue,
        };

        for dev_entry in devices {
            let dev_entry = match dev_entry {
                Ok(v) => v,
                Err(_) => continue,
            };

            let sysfs_path = dev_entry.path();
            let Some(found) = read_dev_numbers_from_sysfs(&sysfs_path) else {
                continue;
            };

            if found == want {
                return Some(sysfs_path);
            }
        }
    }

    None
}

fn classify_kind(device_path: &Path, sysfs_path: Option<&Path>, subsystem: Option<&str>) -> String {
    let dev = device_path.to_string_lossy();

    if dev.starts_with("/dev/pts/") {
        return "pseudo".to_string();
    }

    if dev == "/dev/fuse" {
        return "virtual".to_string();
    }

    if let Some(path) = sysfs_path {
        let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let canon_str = canonical.to_string_lossy();

        if canon_str.starts_with("/sys/devices/virtual/") {
            return "virtual".to_string();
        }

        if canon_str.starts_with("/sys/devices/") {
            return "physical".to_string();
        }
    }

    if let Some(subsys) = subsystem {
        match subsys {
            "tty" => return "virtual".to_string(),
            "misc" => return "virtual".to_string(),
            _ => {}
        }
    }

    "unknown".to_string()
}

pub fn resolve_sysfs_info(device: &DeviceUsage) -> SysfsInfo {
    let Some((major, minor)) = device_numbers_from_devnode(&device.device_path) else {
        return SysfsInfo {
            sysfs_path: None,
            subsystem: None,
            dev_numbers: None,
            driver: None,
            kind: "unknown".to_string(),
        };
    };

    let dev_numbers = Some(format!("{major}:{minor}"));

    let Some(sysfs_path) = find_sysfs_path_by_devnums(major, minor) else {
        let kind = classify_kind(&device.device_path, None, None);

        return SysfsInfo {
            sysfs_path: None,
            subsystem: None,
            dev_numbers,
            driver: None,
            kind,
        };
    };

    let subsystem = read_subsystem_from_sysfs(&sysfs_path);
    let driver = walk_up_for_driver(&sysfs_path);
    let kind = classify_kind(&device.device_path, Some(&sysfs_path), subsystem.as_deref());

    SysfsInfo {
        sysfs_path: Some(sysfs_path),
        subsystem,
        dev_numbers,
        driver,
        kind,
    }
}

pub fn enrich_devices_with_sysfs(usages: &[DeviceUsage]) -> Vec<DeviceRecord> {
    usages
        .iter()
        .cloned()
        .map(|usage| {
            let sysfs = resolve_sysfs_info(&usage);
            DeviceRecord { usage, sysfs }
        })
        .collect()
}