# Configuration

## Preferences

Preferences are stored as JSON:

| Platform | Path |
|----------|------|
| Linux | `~/.config/Colony/Digger/preferences.json` |
| macOS | `~/Library/Application Support/Colony/Digger/preferences.json` |
| Windows | `%LOCALAPPDATA%/Colony/Digger/preferences.json` |

### Available settings

| Setting | Description | Default |
|---------|-------------|---------|
| Theme | Color theme (11 options) | Catppuccin Mocha |
| Accent color | Highlight color (8 options) | Blue |
| Refresh interval | Metric polling rate | 1s |
| Temperature unit | Celsius or Fahrenheit | Celsius |
| Language | UI language (50 options) | English |
| CPU alert threshold | % usage to trigger alert | 90% |
| Memory alert threshold | % usage to trigger alert | 90% |
| Data retention | How long history is kept | 24 hours |
| Font | UI font choice | Auto (language-aware) |
| Auto theme | Match system dark/light mode | Enabled |

## History database

Metrics history is stored in SQLite with WAL mode:

| Platform | Path |
|----------|------|
| Linux | `~/.local/share/digger/history.db` |

The database is pruned automatically based on the data retention setting. History can be exported to CSV or JSON from the History tab.
