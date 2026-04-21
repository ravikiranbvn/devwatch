use crate::model::ProcessRef;
use procfs::process::all_processes;

/// Read VmRSS (resident memory) from /proc/<pid>/status in kB.
fn read_memory_kb(process: &procfs::process::Process) -> Option<u64> {
    let status = process.status().ok()?;
    status.vmrss
}

/// Return a stable list of processes that we can inspect.
pub fn list_processes() -> Vec<ProcessRef> {
    let mut result = Vec::new();

    let processes = match all_processes() {
        Ok(p) => p,
        Err(_) => return result,
    };

    for proc_entry in processes {
        let process = match proc_entry {
            Ok(p) => p,
            Err(_) => continue,
        };

        let pid = process.pid();
        let name = match process.stat() {
            Ok(stat) => stat.comm,
            Err(_) => format!("pid-{pid}"),
        };
        let memory_kb = read_memory_kb(&process);

        result.push(ProcessRef {
            pid,
            name,
            memory_kb,
        });
    }

    result.sort_by_key(|p| p.pid);
    result
}
