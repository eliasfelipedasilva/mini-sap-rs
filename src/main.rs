mod container_governor;
mod ghidra_kernel;

use container_governor::ContainerMemoryGovernor;
use ghidra_kernel::GhidraKernelPatcher;
use std::env;

fn print_help() {
    println!(
        "\
mini-sap-rs — Low-RAM Kernel Patcher & In-Container Memory Governor for SAP NetWeaver 7.52 SP04

Usage:
  ./mini-sap-rs PATCH_NPL    Generate the Ghidra-tuned Low-RAM SAP profile (NPL_D00_vhcalnplci.lowram)
  ./mini-sap-rs --governor   Scan & audit live SAP Kernel (disp+work) and Sybase ASE (dataserver) memory
  ./mini-sap-rs --ghidra     Display the decompiled ab_init (0x00a4b32b) & DbSlExpFuns memory map
"
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(|s| s.to_uppercase()).unwrap_or_default();

    match cmd.as_str() {
        "PATCH_NPL" | "--PATCH" => match GhidraKernelPatcher::patch_npl_profiles() {
            Ok(msg) => println!("{}", msg),
            Err(err) => eprintln!("[PATCH_NPL] Error: {}", err),
        },
        "GOVERNOR" | "--GOVERNOR" => {
            let gov = ContainerMemoryGovernor::scan_and_compact();
            gov.print_report();
        }
        "GHIDRA" | "--GHIDRA" => {
            let patcher = GhidraKernelPatcher::new();
            patcher.print_ghidra_map();
        }
        "--ALL" | "" => {
            let patcher = GhidraKernelPatcher::new();
            patcher.print_ghidra_map();
            println!();
            let gov = ContainerMemoryGovernor::scan_and_compact();
            gov.print_report();
        }
        _ => print_help(),
    }
}
