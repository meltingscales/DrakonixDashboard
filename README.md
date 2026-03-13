# DrakonixDashboard

A terminal dashboard built with Rust and [ratatui](https://github.com/ratatui-org/ratatui).

Displays weather, Google Calendar events, and RSS feeds in a tiling layout — no browser required.

---

## Features

- **Weather** — 7-day forecast columns via [Open-Meteo](https://open-meteo.com/) (no API key needed). Today's column shows current conditions.
- **Google Calendar** — 7-day columnar view of upcoming events via OAuth2. Columns are toggleable. Scroll through events with j/k.
- **RSS** — Aggregates multiple feeds. Toggle individual feeds on/off. Open articles in your default browser with Enter.
- **BSP tiling** — Split panes horizontally or vertically (bspwm-style). Each pane can show any tab independently.
- **No cloud dependencies** — weather is free, calendar uses a one-time OAuth flow that stores a token locally.

---

## Installation

```bash
git clone https://github.com/yourname/DrakonixDashboard
cd DrakonixDashboard
cargo build --release
# binary at: target/release/drakonix
```

---

## Configuration

Copy `.env.example` to `.env` and fill in your values:

```env
# Weather (defaults to New York City if omitted)
WEATHER_LAT=40.71
WEATHER_LON=-74.01
WEATHER_LOCATION_NAME=New York City

# Google Calendar — pick one of the two approaches:

# Option A: point at the credentials.json downloaded from Google Cloud Console
GOOGLE_CREDENTIALS_JSON=/path/to/credentials.json

# Option B: paste the values directly
# GOOGLE_CLIENT_ID=your-client-id.apps.googleusercontent.com
# GOOGLE_CLIENT_SECRET=your-secret

# Calendar to display (default: primary)
GOOGLE_CALENDAR_ID=primary

# Comma-separated RSS feed URLs (optional)
RSS_FEEDS=https://feeds.bbci.co.uk/news/rss.xml,https://hnrss.org/frontpage
```

### Google Calendar setup

1. Go to [console.cloud.google.com](https://console.cloud.google.com/)
2. Enable the **Google Calendar API**
3. Create OAuth credentials → **Desktop app**
4. Download `credentials.json` and point `GOOGLE_CREDENTIALS_JSON` at it (or copy the ID/secret into `.env`)
5. Run the dashboard — your browser will open once for authorization. The token is saved to `~/.drakonix_gcal.json` and refreshed automatically.

---

## Controls

| Key | Action |
|-----|--------|
| `1` / `2` / `3` | Switch focused pane to Weather / Calendar / RSS |
| `←` / `→` | Cycle tab in focused pane |
| `Tab` / `Shift+Tab` | Focus next / previous pane |
| `\|` or `\` | Split pane side-by-side |
| `-` | Split pane top / bottom |
| `x` | Close focused pane |
| `r` | Refresh focused pane |
| `?` | Toggle help overlay |
| `q` / `Esc` | Quit (or close modal/help) |

**Within RSS tab:**

| Key | Action |
|-----|--------|
| `j` / `↓` | Next item |
| `k` / `↑` | Previous item |
| `Enter` | Open article in browser |
| `f` | Open feed sources modal (Space to toggle) |

**Within Calendar tab:**

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll events down |
| `k` / `↑` | Scroll events up |
| `d` | Open day columns modal (Space to toggle) |

---

## Architecture

```
src/
├── main.rs          # tokio runtime, terminal setup, event loop
├── app.rs           # App state, key handling, background task dispatch
├── config.rs        # .env / credentials.json loading
├── tiling.rs        # BSP tile tree (split, close, focus)
├── api/
│   ├── weather.rs   # Open-Meteo fetch + WMO code mapping
│   ├── calendar.rs  # Google OAuth2 flow + Calendar API fetch
│   └── rss.rs       # Multi-feed RSS/Atom aggregation
└── ui/
    ├── mod.rs        # Root render, tile recursion, help overlay, tab bar
    └── tabs/
        ├── weather.rs   # 7-column forecast layout
        ├── calendar.rs  # 7-column event layout + day picker modal
        └── rss.rs       # Feed list + detail pane + feed picker modal
```

API calls run in `tokio::spawn` tasks and send results back via `mpsc::sync_channel`. The main loop polls for updates every 100 ms and re-renders.

---

## License

MIT
