# Digger

**Digger** is the system monitor for [Colony](https://github.com/Project-Colony/Colony). Built with Rust and [Iced](https://iced.rs), it provides real-time visibility into your machine's health - CPU, memory, network, disk, GPU, and processes - with a clean, themeable interface.

## What it does

Digger gives you a live dashboard of everything happening on your system:

- **System metrics** - CPU (per-core and global), memory, swap, network I/O, disk I/O, temperatures, load averages
- **GPU monitoring** - see [GPU support by platform](#gpu-support-by-platform)
- **Process management** - List, filter, sort, group, and kill processes. Processes are classified into Apps, Background, and System categories
- **Persistent history** - All metrics are stored in a local SQLite database with configurable retention, exportable to CSV or JSON
- **Alerting** - Configurable CPU and memory thresholds with desktop notifications and an event log

## Look & feel

Digger ships with **11 color themes** across 4 families - Catppuccin, Gruvbox, Everblush, and Kanagawa - each combinable with **8 accent colors**. Dark mode is detected automatically.

The UI is organized into four tabs:

| Tab | Purpose |
|-----|---------|
| **Overview** | Gauges, charts, and sparklines for key metrics at a glance |
| **Processes** | Full process table with search, sorting, and grouping |
| **History** | Time-series charts with selectable ranges (1m → 24h) |
| **Event Log** | Alerts and anomalies with severity levels |

## Internationalization

Digger supports **50 languages** with zero-cost static string tables compiled directly into the binary. Font selection adapts automatically to the active language.

## GPU support by platform

- **Linux**: every GPU the kernel exposes under `/sys/class/drm` (AMD, Intel, NVIDIA), with utilization, VRAM, temperature and power where the driver reports them. NVIDIA cards are completed with `nvidia-smi` when it is installed.
- **Windows**: NVIDIA cards through `nvidia-smi` when it is installed. Otherwise (AMD, Intel) Digger falls back to WMI, which is coarser: utilization and memory are summed across all adapters, and temperature comes from LibreHardwareMonitor or OpenHardwareMonitor if one is running, else from the hottest system thermal zone.
- **macOS**: no GPU monitoring.

NVML is only used by builds made with `--features gpu`; the release binaries are built without it.

## Running the binaries

The release binaries are not signed with an Apple or Microsoft certificate, so each OS warns on first launch. Colony installs them for you and checks the Project-Colony signature of every asset; the steps below are for running a binary downloaded by hand.

- **Linux**: `chmod +x d1gg2r-linux && ./d1gg2r-linux`
- **macOS**: make it executable and clear the download quarantine flag, then run it:

  ```bash
  chmod +x d1gg2r-macos
  xattr -d com.apple.quarantine d1gg2r-macos
  ./d1gg2r-macos
  ```

  `xattr` reports "No such xattr" when the file was not quarantined, which is fine. Use `d1gg2r-macos-x86` on Intel Macs. If you skip the `xattr` step, macOS 15 and later block the first launch; allow it under **System Settings > Privacy & Security > Open Anyway**.
- **Windows**: SmartScreen shows "Windows protected your PC". Click **More info**, then **Run anyway**.

## Screenshots

*Coming soon*

## Documentation

Build instructions, architecture details, and configuration reference are in the [`docs/`](docs/) folder.

## License

Digger is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. The full text is in [LICENSE](LICENSE).

The font files bundled under `src/ui/assets/fonts/` are third-party and remain under their own licenses.
