# mini-sap-rs 🦀

**Run Mini SAP (SAP NetWeaver AS ABAP 7.52 SP04 + Sybase ASE 16.0 + OData V2 Gateway) in < 3 GB RAM.**

`mini-sap-rs` is a zero-dependency Rust kernel patcher and in-container memory governor designed to run the full **SAP NetWeaver 7.52 SP04 Docker container** ([`yoraco/sap-dev-edition-7.52-sp04:latest`](https://hub.docker.com/r/yoraco/sap-dev-edition-7.52-sp04)) inside **1.79 GiB – 2.23 GiB of physical RAM** on a 16 GB Linux laptop — with real **SAP GUI (`:3200`)**, **ALV Grid**, **Sybase ASE 16.0 (`NPL`)**, and **SAP Gateway OData V2 (`:8000`)**.

---

## Memory Footprint Benchmark

| Environment | Physical RAM Usage | 16 GB Laptop Behavior |
|---|---|---|
| **SAP ABAP Trial (HANA)** | **16.0 GiB – 22.0 GiB** | Heavy SSD swap & work-process boot timeouts |
| **SAP NetWeaver 7.52 SP04 (Sybase ASE — Stock Container)** | **4.2 GiB – 5.8 GiB** | Boots, but consumes ~35% of host memory |
| **SAP NetWeaver 7.52 SP04 (Container + `mini-sap-rs`)** | **1.79 GiB (boot) / 2.23 GiB (SAP GUI + ALV + OData)** | **Runs 100% in RAM with >13 GB free on host** |

---

## Quick Start

### Prerequisites
- **Docker** with the SAP NetWeaver 7.52 SP04 base image:
  ```bash
  docker pull yoraco/sap-dev-edition-7.52-sp04:latest
  ```
- **Rust** (`cargo` / `rustc` 1.70+)
- **SAP GUI for Java** (connects to `127.0.0.1:3200`, Instance `00`, Client `001`, User `DEVELOPER`, Password `Down1oad`)

### Build & Run
```bash
# 1. Build a static Rust binary (compatible with the container's OpenSUSE Leap glibc)
rustc --edition 2021 -O -C target-feature=+crt-static src/main.rs -o mini-sap-rs

# 2. Generate the Ghidra-tuned Low-RAM SAP Kernel profile (NPL_D00_vhcalnplci.lowram)
./mini-sap-rs PATCH_NPL

# 3. Provision a fresh container or start an existing Low-RAM container
#    (mounts the Low-RAM profile and injects /usr/local/bin/mini-sap-rs into the container)
./start-container-lowram.sh
# or start an already-provisioned container (ASCS01 + D00 + Sybase ASE 16.0 + OData :8000):
./start-sap-lowram.sh

# 4. Run the in-container Rust Memory Governor inside the running SAP container
docker exec sap-npl-lowram /usr/local/bin/mini-sap-rs --governor

# 5. (Optional) Install your free SAP NetWeaver ABAP evaluation license (NPL.txt)
./install-abap-license.sh ~/Downloads/NPL.txt
```

---

## How It Works

### 1. Kernel Boot Allocation Mapping (`disp+work` / `ab_init @ 0x00a4b32b`)
By decompiling the SAP 753 kernel binary (`disp+work`, 87 MB ELF, 172,919 symbols) and Sybase driver (`dbsdbslib.so`, `DbSlExpFuns @ 0x001ff470`) with **NSA Ghidra Headless**, `mini-sap-rs` targets the exact memory pools eagerly allocated by every work process during `ThStart -> ab_init()`:

- **`abap/shared_objects_size_MB`**: Reduced from `386 MB` to **`64 MB`** (`_Z10ab_shminitv`).
- **`abap/buffersize`**: Reduced from `200,000 KB` to **`80,000 KB`** (`_Z9ab_pbinitv`).
- **`PHYS_MEMSIZE`**: Capped at **`1280 MB`** so Extended Memory (`em/initial_size_MB`) does not scale up to host RAM.
- **Work Process Pool**: Tuned to the exact minimum required to pass SAP `SICK` installation checks (`2 DIA`, `1 UPD`, `1 BTC`, `1 SPO`).
- **Background Trace Monitors & IGS**: Disables eager boot monitors (`rtm/enable = OFF`, `abap/cov_enable = 0`) and the multiprocess Internet Graphics Server (`igswd_mt`).

### 2. Project Structure
- [`src/ghidra_kernel.rs`](src/ghidra_kernel.rs) — Maps the `ab_init` (`0x00a4b32b`) and `DbSlExpFuns` (`0x001ff470`) subsystems extracted via Ghidra and generates `NPL_D00_vhcalnplci.lowram` + `install-lowram.sh` (`PATCH_NPL`).
- [`src/container_governor.rs`](src/container_governor.rs) — In-container Rust memory inspector & governor (`/usr/local/bin/mini-sap-rs --governor`) injected into `sap-npl-lowram` to audit `VmRSS`, private anonymous heap (`RssAnon`), and shared IPC buffers (`RssShmem`) across all live SAP `disp+work`, `icman`, `gwrd`, and Sybase `dataserver` processes.
- [`src/main.rs`](src/main.rs) — CLI entrypoint (`PATCH_NPL`, `--governor`, `--ghidra`).
- [`tools/DecompileSapKernel.java`](tools/DecompileSapKernel.java) & [`tools/ExportSymbolsAndDecompile.java`](tools/ExportSymbolsAndDecompile.java) — Headless Ghidra scripts used to decompile `disp+work`, `dbsdbslib.so`, and `rfcexec`.
- [`docs/architecture.html`](docs/architecture.html) — Editorial SVG architecture diagram (`diagram-design`) of the Ghidra + Rust + Docker SAP 7.52 pipeline.

---

## PT-BR — Resumo

O **`mini-sap-rs`** permite rodar o container Docker oficial do **SAP NetWeaver 7.52 SP04 (`yoraco/sap-dev-edition-7.52-sp04` — ABAP + Sybase ASE 16.0 + SAP GUI + ALV Grid + SAP Gateway OData V2)** consumindo apenas **~1,79 GiB no boot e 2,23 GiB em uso** em notebooks Linux de 16 GB, sem travar em swap:
- **`./mini-sap-rs PATCH_NPL`:** Gera o profile enxuto baseado na engenharia reversa da função `ab_init()` (`0x00a4b32b`) no Ghidra, cortando **1,47 GB** de alocações ociosas no boot dos Work Processes.
- **`/usr/local/bin/mini-sap-rs --governor`:** Binário estático em Rust injetado dentro do container `sap-npl-lowram` que audita e monitora o Heap Anônimo (`RssAnon`) e a Memória Compartilhada IPC (`RssShmem`) dos 16 processos reais do SAP e do Sybase ASE.

---

## License
MIT
