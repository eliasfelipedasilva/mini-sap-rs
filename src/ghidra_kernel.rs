use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct AbInitSubsystem {
    pub symbol: &'static str,
    pub addr: &'static str,
    pub param: &'static str,
    pub stock_sap_kb: usize,
    pub lowram_sap_kb: usize,
    pub role: &'static str,
}

#[derive(Debug, Clone)]
pub struct DbslSlot {
    pub index: usize,
    pub symbol: &'static str,
    pub operation: &'static str,
}

pub struct GhidraKernelPatcher {
    pub subsystems: Vec<AbInitSubsystem>,
    pub dbsl_vtable: Vec<DbslSlot>,
}

impl GhidraKernelPatcher {
    pub fn new() -> Self {
        Self {
            subsystems: vec![
                AbInitSubsystem {
                    symbol: "DpLoopInit / DpShmPrfCreate",
                    addr: "0x00642011",
                    param: "PHYS_MEMSIZE",
                    stock_sap_kb: 1_979_392,
                    lowram_sap_kb: 1_310_720,
                    role: "Dispatcher Shared Memory & Extended Memory Pool",
                },
                AbInitSubsystem {
                    symbol: "_Z10ab_shminitv",
                    addr: "0x00a4b840",
                    param: "abap/shared_objects_size_MB",
                    stock_sap_kb: 395_264,
                    lowram_sap_kb: 65_536,
                    role: "ABAP Shared Objects Memory Pool",
                },
                AbInitSubsystem {
                    symbol: "_Z9ab_pbinitv",
                    addr: "0x00a4b4f0",
                    param: "abap/buffersize",
                    stock_sap_kb: 200_000,
                    lowram_sap_kb: 80_000,
                    role: "ABAP Program Buffer (PXA)",
                },
                AbInitSubsystem {
                    symbol: "ab_ltypddic / ab_ltypload",
                    addr: "0x00a4b710",
                    param: "rsdb/ntab/ftabsize",
                    stock_sap_kb: 128_000,
                    lowram_sap_kb: 15_000,
                    role: "DDIC Type & Load Rollbuffers (24 ab_l* segments)",
                },
                AbInitSubsystem {
                    symbol: "_Z11ab_cov_initv / _Z11ab_rtm_initv",
                    addr: "0x00a4b960",
                    param: "rtm/enable = OFF",
                    stock_sap_kb: 96_000,
                    lowram_sap_kb: 0,
                    role: "Coverage (SCOV), SQL Monitor (SQLM) & Runtime Monitor (SRTM)",
                },
                AbInitSubsystem {
                    symbol: "Start_Program_04 (igswd_mt)",
                    addr: "profile:D00",
                    param: "# Start_Program_04",
                    stock_sap_kb: 180_000,
                    lowram_sap_kb: 0,
                    role: "Multiprocess Internet Graphics Server (IGS)",
                },
            ],
            dbsl_vtable: vec![
                DbslSlot { index: 0, symbol: "DbSlSdbControl", operation: "DBSL_CMDTYPE dispatcher (op=0xc in ab_init)" },
                DbslSlot { index: 1, symbol: "DbSlSdbConnect", operation: "Sybase ASE 16.0 connection on port 4901" },
                DbslSlot { index: 2, symbol: "DbSlSdbDisconnect", operation: "Close DB connection" },
                DbslSlot { index: 6, symbol: "DbSlSdbExecute", operation: "Execute native DDL/DML SQL" },
                DbslSlot { index: 9, symbol: "DbSlSdbRead", operation: "OpenSQL single-row/cursor read" },
                DbslSlot { index: 29, symbol: "DbSlSdbBegReadFdaVers", operation: "Fast Data Access (FDA) SELECT FOR ALL ENTRIES" },
            ],
        }
    }

    pub fn patch_npl_profiles() -> Result<String, String> {
        let profile_dir = Path::new("sap_kernel_bins/sapmnt/NPL/profile");
        let orig_d00 = profile_dir.join("NPL_D00_vhcalnplci");
        if !orig_d00.exists() {
            return Err(format!("Original profile not found at {:?}", orig_d00));
        }

        let content = fs::read_to_string(&orig_d00).map_err(|e| e.to_string())?;
        let mut patched_lines = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("rdisp/wp_no_btc") {
                patched_lines.push("rdisp/wp_no_btc = 1".to_string());
            } else if trimmed.starts_with("rdisp/wp_no_spo") {
                patched_lines.push("rdisp/wp_no_spo = 1".to_string());
            } else if trimmed.starts_with("PHYS_MEMSIZE") {
                patched_lines.push("PHYS_MEMSIZE = 1280".to_string());
            } else if trimmed.starts_with("abap/shared_objects_size_MB") {
                patched_lines.push("abap/shared_objects_size_MB = 64".to_string());
            } else if trimmed.starts_with("abap/buffersize") {
                patched_lines.push("abap/buffersize = 80000".to_string());
            } else if trimmed.starts_with("Start_Program_04") {
                patched_lines.push("# [LOW-RAM RUST PATCH] Disable eager IGS daemon: ".to_string() + line);
            } else {
                patched_lines.push(line.to_string());
            }
        }

        patched_lines.push("\n# --- [MINI-SAP-RS GHIDRA LOW-RAM TUNING (< 3 GB CONTAINER RAM)] ---".to_string());
        patched_lines.push("em/initial_size_MB = 512".to_string());
        patched_lines.push("abap/heap_area_dia = 500000000".to_string());
        patched_lines.push("abap/heap_area_total = 800000000".to_string());
        patched_lines.push("rsdb/ntab/entrycount = 10000".to_string());
        patched_lines.push("rsdb/ntab/ftabsize = 15000".to_string());
        patched_lines.push("rtm/enable = OFF".to_string());

        let out_d00 = profile_dir.join("NPL_D00_vhcalnplci.lowram");
        fs::write(&out_d00, patched_lines.join("\n") + "\n").map_err(|e| e.to_string())?;

        let orig_inst = Path::new("sap_kernel_bins/install-quiet.sh");
        if orig_inst.exists() {
            let inst_src = fs::read_to_string(orig_inst).map_err(|e| e.to_string())?;
            let inst_patched = inst_src
                .replace("tar -zpxvf - -C /", "tar -zpxf - -C /")
                .replace(
                    "./saphostexec -install || do_exit $ERR_install_saphost",
                    "./saphostexec -install &\n    sleep 30",
                )
                .replace(
                    "rm -rf /tmp/hostctrl || log_echo \"Failed to clean up temporary directory\"",
                    "# rm -rf /tmp/hostctrl || log_echo \"Failed to clean up temporary directory\"",
                )
                .replace(
                    "set_profile_param \"PHYS_MEMSIZE\" \"2048\" /sapmnt/${SID}/profile/${SID}_D00_*",
                    "set_profile_param \"PHYS_MEMSIZE\" \"1280\" /sapmnt/${SID}/profile/${SID}_D00_*\n    set_profile_param \"rdisp/wp_no_btc\" \"1\" /sapmnt/${SID}/profile/${SID}_D00_*\n    set_profile_param \"rdisp/wp_no_spo\" \"1\" /sapmnt/${SID}/profile/${SID}_D00_*\n    set_profile_param \"abap/buffersize\" \"80000\" /sapmnt/${SID}/profile/${SID}_D00_*",
                )
                .replace(
                    "set_profile_param \"abap/shared_objects_size_MB\" \"386\" /sapmnt/${SID}/profile/${SID}_D00_*",
                    "set_profile_param \"abap/shared_objects_size_MB\" \"64\" /sapmnt/${SID}/profile/${SID}_D00_*",
                );
            fs::write("sap_kernel_bins/install-lowram.sh", inst_patched).map_err(|e| e.to_string())?;
        }

        Ok(format!(
            "[MINI-SAP-RS] Low-RAM SAP Kernel profile generated successfully:\n  • {:?}\n  • sap_kernel_bins/install-lowram.sh",
            out_d00
        ))
    }

    pub fn print_ghidra_map(&self) {
        println!("┌────────────────────────────────────────────────────────────────────────────────────────┐");
        println!("│ GHIDRA SAP 7.52 KERNEL MEMORY MAP (`disp+work` @ 0x00642011 / `ab_init` @ 0x00a4b32b)  │");
        println!("├────────────┬───────────────────────────────┬──────────────┬──────────────┬─────────────┤");
        println!("│ ADDRESS    │ KERNEL SYMBOL                 │ STOCK SAP KB │ LOW-RAM KB   │ SAVED       │");
        println!("├────────────┼───────────────────────────────┼──────────────┼──────────────┼─────────────┤");
        let mut stock_sum = 0usize;
        let mut low_sum = 0usize;
        for s in &self.subsystems {
            stock_sum += s.stock_sap_kb;
            low_sum += s.lowram_sap_kb;
            let saved_mb = (s.stock_sap_kb.saturating_sub(s.lowram_sap_kb)) as f64 / 1024.0;
            println!(
                "│ {:<10} │ {:<29} │ {:>9} KB │ {:>9} KB │ {:>8.1} MB │",
                s.addr,
                &s.symbol[..s.symbol.len().min(29)],
                s.stock_sap_kb,
                s.lowram_sap_kb,
                saved_mb
            );
        }
        println!("├────────────┴───────────────────────────────┴──────────────┴──────────────┴─────────────┤");
        println!(
            "│ TOTAL KERNEL POOL FOOTPRINT                │ {:>9.1} MB │ {:>9.1} MB │ {:>8.1} MB │",
            stock_sum as f64 / 1024.0,
            low_sum as f64 / 1024.0,
            (stock_sum.saturating_sub(low_sum)) as f64 / 1024.0
        );
        println!("├────────────────────────────────────────────┴──────────────┴──────────────┴─────────────┤");
        println!("│ Sybase ASE 16.0 DBSL Vtable (`dbsdbslib.so` -> `DbSlExpFuns` @ 0x001ff470, 35 slots):  │");
        for slot in &self.dbsl_vtable {
            println!(
                "│   slot[{:>2}] = {:<24} -> {:<45} │",
                slot.index, slot.symbol, slot.operation
            );
        }
        println!("└────────────────────────────────────────────────────────────────────────────────────────┘");
    }
}
