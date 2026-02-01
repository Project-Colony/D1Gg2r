# Digger

**Digger** is the system monitor for [Colony](https://github.com/MotherSphere). Built with Rust and [Iced](https://iced.rs), it provides real-time visibility into your machine's health — CPU, memory, network, disk, GPU, and processes — with a clean, themeable interface.

## What it does

Digger gives you a live dashboard of everything happening on your system:

- **System metrics** — CPU (per-core and global), memory, swap, network I/O, disk I/O, temperatures, load averages
- **GPU monitoring** — NVIDIA (NVML), AMD and Intel (sysfs), with automatic backend detection
- **Process management** — List, filter, sort, group, and kill processes. Processes are classified into Apps, Background, and System categories
- **Persistent history** — All metrics are stored in a local SQLite database with configurable retention, exportable to CSV or JSON
- **Alerting** — Configurable CPU and memory thresholds with desktop notifications and an event log

## Look & feel

Digger ships with **11 color themes** across 4 families — Catppuccin, Gruvbox, Everblush, and Kanagawa — each combinable with **8 accent colors**. Dark mode is detected automatically.

The UI is organized into four tabs:

| Tab | Purpose |
|-----|---------|
| **Overview** | Gauges, charts, and sparklines for key metrics at a glance |
| **Processes** | Full process table with search, sorting, and grouping |
| **History** | Time-series charts with selectable ranges (1m → 24h) |
| **Event Log** | Alerts and anomalies with severity levels |

## Internationalization

Digger supports **50 languages** with zero-cost static string tables compiled directly into the binary. Font selection adapts automatically to the active language:

| Font | Coverage |
|------|----------|
| Iosevka Nerd Font | Latin, symbols (default) |
| Sarasa Mono Nerd Font | Chinese, Japanese, Korean |
| DejaVu Sans Mono NF | Arabic, Persian |
| Noto Mono NF | Hindi, Bengali, Tamil, and other Indic scripts |
| OpenDyslexic | Accessibility |

## Screenshots

*Coming soon*

## Documentation

Build instructions, architecture details, and configuration reference are in the [`docs/`](docs/) folder.

## License

MIT
