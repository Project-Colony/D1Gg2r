# Architecture

## Source layout

```
src/
├── main.rs          — Entry point, font embedding, Iced app bootstrap
├── ui.rs            — UI views, state management, message handling
├── metrics.rs       — System metrics collection via sysinfo
├── history.rs       — SQLite persistence with WAL mode
├── preferences.rs   — JSON-based user preferences (serde)
├── chart.rs         — Canvas-based line chart rendering
├── gauge.rs         — Radial gauge and sparkline components
├── gpu.rs           — GPU monitoring (multi-backend: NVML, sysfs, CLI, WMI)
├── theme.rs         — 11 themes × 8 accent color palette system
├── i18n.rs          — 50 languages with static string tables
├── icons.rs         — Nerd Font icon constants
└── ringbuf.rs       — Fixed-capacity ring buffer for live data
```

## Key data structures

### Snapshot

A complete capture of all system metrics at a point in time: CPU (per-core + global), memory, swap, disk I/O, network I/O, temperatures, processes, GPU state, load averages, and static system info (OS, kernel, hostname).

### LivePoint

A lightweight struct used for rolling charts. Contains only CPU %, memory %, network RX/TX, and disk read/write — no heap allocations.

### ProcessInfo

Full process details: PID, parent PID, name, command args, CPU/memory usage, virtual memory, UID, thread count, status, and desktop app classification.

## Design patterns

- **Zero-cost i18n** — All translated strings are `&'static str`, resolved at compile time.
- **Ring buffer** — Fixed-capacity circular buffer for live chart data, avoids allocations during updates.
- **Canvas rendering** — Charts, gauges, and sparklines are drawn directly on the Iced canvas.
- **Multi-backend GPU** — Detection cascades from NVML → sysfs → nvidia-smi CLI → WMI.
- **Arc-based system info** — Static info (hostname, OS, kernel) is shared via `Arc` to avoid repeated allocations.
- **Message-driven UI** — State management follows Iced's Elm-like architecture with typed messages.

## Dependencies

| Crate | Purpose |
|-------|---------|
| `iced` 0.13 | GUI framework (canvas, tokio) |
| `sysinfo` 0.32 | Cross-platform system information |
| `rusqlite` 0.32 | SQLite with bundled support |
| `chrono` 0.4 | Date/time handling |
| `serde` / `serde_json` | Serialization |
| `notify-rust` 4 | Desktop notifications |
| `nvml-wrapper` 0.10 | NVIDIA GPU (optional, feature-gated) |
| `wmi` 0.15 | Windows GPU detection (Windows only) |
