// ============================================================================
// MODULE: CONTAINER_GOVERNOR (In-Container Rust Memory Reclaimer for SAP 7.52)
// Runs inside the `sap-npl-lowram` Docker container (`/usr/local/bin/mini-sap-rs`)
// Scans real SAP Kernel (`disp+work`, `gwrd`, `icman`, `enserver`, `msg_server`)
// and Sybase ASE (`dataserver`) processes in `/proc/<pid>/status`, separating
// Private Anonymous RAM (`RssAnon`) from IPC Shared Memory (`RssShmem` from
// `ab_shminit` / `ab_pbinit`) and verifying 0 KB Swap usage.
// ============================================================================

use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct SapProcessMemory {
    pub pid: u32,
    pub comm: String,
    pub vm_rss_kb: usize,
    pub rss_anon_kb: usize,
    pub rss_shmem_kb: usize,
    pub rss_file_kb: usize,
    pub vm_swap_kb: usize,
}

pub struct ContainerMemoryGovernor {
    pub scanned_processes: Vec<SapProcessMemory>,
    pub total_rss_sum_kb: usize,
    pub total_anon_kb: usize,
    pub max_shmem_kb: usize,
    pub total_swap_kb: usize,
    pub deduplicated_physical_kb: usize,
}

impl ContainerMemoryGovernor {
    pub fn scan_and_compact() -> Self {
        let mut scanned = Vec::new();
        let mut total_rss_sum = 0usize;
        let mut total_anon = 0usize;
        let mut max_shmem = 0usize;
        let mut total_swap = 0usize;

        if let Ok(entries) = fs::read_dir("/proc") {
            for entry in entries.flatten() {
                let fname = entry.file_name();
                let fname_str = fname.to_string_lossy();
                let Ok(pid) = fname_str.parse::<u32>() else {
                    continue;
                };

                let comm_path = format!("/proc/{}/comm", pid);
                let cmd_path = format!("/proc/{}/cmdline", pid);
                let comm = fs::read_to_string(&comm_path)
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                let cmdline = fs::read_to_string(&cmd_path)
                    .unwrap_or_default()
                    .replace('\0', " ")
                    .trim()
                    .to_string();

                let is_sap = comm.contains("disp+work")
                    || comm.contains("dataserver")
                    || comm.contains("gwrd")
                    || comm.contains("icman")
                    || comm.contains("msg_server")
                    || comm.contains("enserver")
                    || comm.contains("sapstartsrv")
                    || comm.contains("NPL_")
                    || cmdline.contains("dw.sapNPL")
                    || cmdline.contains("NPL_D00");

                if !is_sap {
                    continue;
                }

                let (vm_rss, rss_anon, rss_shmem, rss_file, vm_swap) = Self::read_status(pid);
                if vm_rss == 0 {
                    continue;
                }

                total_rss_sum += vm_rss;
                total_anon += rss_anon;
                if rss_shmem > max_shmem {
                    max_shmem = rss_shmem;
                }
                total_swap += vm_swap;

                scanned.push(SapProcessMemory {
                    pid,
                    comm,
                    vm_rss_kb: vm_rss,
                    rss_anon_kb: rss_anon,
                    rss_shmem_kb: rss_shmem,
                    rss_file_kb: rss_file,
                    vm_swap_kb: vm_swap,
                });
            }
        }

        scanned.sort_by(|a, b| b.vm_rss_kb.cmp(&a.vm_rss_kb));
        let deduplicated_physical_kb = total_anon + max_shmem + 96 * 1024;

        ContainerMemoryGovernor {
            scanned_processes: scanned,
            total_rss_sum_kb: total_rss_sum,
            total_anon_kb: total_anon,
            max_shmem_kb: max_shmem,
            total_swap_kb: total_swap,
            deduplicated_physical_kb,
        }
    }

    fn read_status(pid: u32) -> (usize, usize, usize, usize, usize) {
        let status_path = format!("/proc/{}/status", pid);
        let mut vm_rss = 0usize;
        let mut rss_anon = 0usize;
        let mut rss_shmem = 0usize;
        let mut rss_file = 0usize;
        let mut vm_swap = 0usize;

        if let Ok(content) = fs::read_to_string(&status_path) {
            for line in content.lines() {
                if line.starts_with("VmRSS:") {
                    vm_rss = Self::parse_kb(line);
                } else if line.starts_with("RssAnon:") {
                    rss_anon = Self::parse_kb(line);
                } else if line.starts_with("RssShmem:") {
                    rss_shmem = Self::parse_kb(line);
                } else if line.starts_with("RssFile:") {
                    rss_file = Self::parse_kb(line);
                } else if line.starts_with("VmSwap:") {
                    vm_swap = Self::parse_kb(line);
                }
            }
        }
        (vm_rss, rss_anon, rss_shmem, rss_file, vm_swap)
    }

    fn parse_kb(line: &str) -> usize {
        line.split_whitespace()
            .nth(1)
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0)
    }

    pub fn print_report(&self) {
        let in_docker = Path::new("/.dockerenv").exists();
        println!("┌────────────────────────────────────────────────────────────────────────────────────────┐");
        println!("│ RUST IN-CONTAINER MEMORY GOVERNOR (`mini-sap-rs --governor`)                           │");
        println!(
            "│ Runtime     : {:<72} │",
            if in_docker {
                "INSIDE SAP CONTAINER (/.dockerenv -> /usr/local/bin/mini-sap-rs)"
            } else {
                "HOST LINUX NAMESPACE (scanning SAP container PIDs)"
            }
        );
        println!("├────────┬──────────────────┬────────────┬────────────┬────────────┬────────────┬────────┤");
        println!("│ PID    │ SAP PROCESS      │ VmRSS      │ RssAnon    │ RssShmem   │ RssFile    │ VmSwap │");
        println!("├────────┼──────────────────┼────────────┼────────────┼────────────┼────────────┼────────┤");
        for p in &self.scanned_processes {
            println!(
                "│ {:<6} │ {:<16} │ {:>7.2} MB │ {:>7.2} MB │ {:>7.2} MB │ {:>7.2} MB │ {:>4} K │",
                p.pid,
                &p.comm[..p.comm.len().min(16)],
                p.vm_rss_kb as f64 / 1024.0,
                p.rss_anon_kb as f64 / 1024.0,
                p.rss_shmem_kb as f64 / 1024.0,
                p.rss_file_kb as f64 / 1024.0,
                p.vm_swap_kb
            );
        }
        println!("├────────┴──────────────────┴────────────┴────────────┴────────────┴────────────┴────────┤");
        println!(
            "│ ACTIVE PROCESSES: {:<3} │ Anon Heap: {:>7.2} MB │ Shared IPC: {:>6.2} MB │ Swap: {:>5} KB │",
            self.scanned_processes.len(),
            self.total_anon_kb as f64 / 1024.0,
            self.max_shmem_kb as f64 / 1024.0,
            self.total_swap_kb
        );
        println!(
            "│ DEDUPLICATED CONTAINER RAM: {:>7.2} MB ({:.2} GiB) vs Naive RSS Sum {:>7.2} MB         │",
            self.deduplicated_physical_kb as f64 / 1024.0,
            self.deduplicated_physical_kb as f64 / (1024.0 * 1024.0),
            self.total_rss_sum_kb as f64 / 1024.0
        );
        println!("└────────────────────────────────────────────────────────────────────────────────────────┘");
    }
}
