use std::sync::mpsc;
use std::time::{Duration, Instant};

use crossterm::event::KeyCode;

use crate::api::{calendar::CalendarEvent, rss::RssItem, weather::WeatherData, ApiUpdate};
use crate::config::Config;
use crate::tiling::{SplitDir, TileLayout};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Weather,
    Calendar,
    Rss,
}

impl Tab {
    pub const ALL: &'static [Tab] = &[Tab::Weather, Tab::Calendar, Tab::Rss];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Weather => "Weather",
            Tab::Calendar => "Google Calendar",
            Tab::Rss => "RSS",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Tab::Weather => 0,
            Tab::Calendar => 1,
            Tab::Rss => 2,
        }
    }

    pub fn from_index(i: usize) -> Tab {
        Tab::ALL[i % Tab::ALL.len()]
    }
}

pub struct TabState<T> {
    pub data: Option<T>,
    pub error: Option<String>,
    pub loading: bool,
    pub last_refresh: Option<Instant>,
}

impl<T> Default for TabState<T> {
    fn default() -> Self {
        Self {
            data: None,
            error: None,
            loading: false,
            last_refresh: None,
        }
    }
}

impl<T> TabState<T> {
    fn needs_refresh(&self) -> bool {
        !self.loading
            && (self.last_refresh.is_none()
                || self.last_refresh.unwrap().elapsed() > Duration::from_secs(300))
    }

    fn set_loading(&mut self) {
        self.loading = true;
        self.error = None;
    }
}

pub struct App {
    /// BSP tile layout — default is a single pane.
    pub tiles: TileLayout,
    /// Set while waiting for the user to authorize in their browser.
    pub calendar_auth_url: Option<String>,
    pub show_help: bool,
    pub weather: TabState<WeatherData>,
    pub calendar: TabState<Vec<CalendarEvent>>,
    pub rss: TabState<Vec<RssItem>>,
    pub rss_selected: usize,
    /// Parallel to `config.rss_feeds` — which feeds are active.
    pub rss_feed_enabled: Vec<bool>,
    pub rss_feed_picker_open: bool,
    pub rss_feed_cursor: usize,
    /// Which of the 7 upcoming days to show in the calendar columnar view.
    pub calendar_days_enabled: [bool; 7],
    pub calendar_day_picker_open: bool,
    pub calendar_day_cursor: usize,
    pub calendar_scroll: u16,
    pub config: Config,
    tx: mpsc::SyncSender<ApiUpdate>,
    rx: mpsc::Receiver<ApiUpdate>,
}

impl App {
    pub fn new(config: Config) -> Self {
        let (tx, rx) = mpsc::sync_channel(64);
        App {
            tiles: TileLayout::new(Tab::Weather),
            calendar_auth_url: None,
            show_help: false,
            weather: TabState::default(),
            calendar: TabState::default(),
            rss: TabState::default(),
            rss_selected: 0,
            rss_feed_enabled: vec![true; config.rss_feeds.len()],
            rss_feed_picker_open: false,
            rss_feed_cursor: 0,
            calendar_days_enabled: [true; 7],
            calendar_day_picker_open: false,
            calendar_day_cursor: 0,
            calendar_scroll: 0,
            config,
            tx,
            rx,
        }
    }

    pub fn focused_tab(&self) -> Tab {
        self.tiles.focused_tab()
    }

    pub fn process_updates(&mut self) {
        while let Ok(update) = self.rx.try_recv() {
            match update {
                ApiUpdate::Weather(data) => {
                    self.weather.data = Some(data);
                    self.weather.loading = false;
                    self.weather.error = None;
                    self.weather.last_refresh = Some(Instant::now());
                }
                ApiUpdate::WeatherError(e) => {
                    self.weather.error = Some(e);
                    self.weather.loading = false;
                }
                ApiUpdate::Rss(items) => {
                    self.rss.data = Some(items);
                    self.rss.loading = false;
                    self.rss.error = None;
                    self.rss.last_refresh = Some(Instant::now());
                    self.rss_selected = 0;
                }
                ApiUpdate::RssError(e) => {
                    self.rss.error = Some(e);
                    self.rss.loading = false;
                }
                ApiUpdate::CalendarNeedAuth(url) => {
                    // Auth flow in progress — show URL in the UI as a fallback
                    self.calendar_auth_url = Some(url);
                }
                ApiUpdate::Calendar(events) => {
                    self.calendar.data = Some(events);
                    self.calendar.loading = false;
                    self.calendar.error = None;
                    self.calendar.last_refresh = Some(Instant::now());
                    self.calendar_auth_url = None;
                    self.calendar_scroll = 0;
                }
                ApiUpdate::CalendarError(e) => {
                    self.calendar.error = Some(e);
                    self.calendar.loading = false;
                    self.calendar_auth_url = None;
                }
            }
        }
    }

    /// Refresh all currently visible panes that need it.
    pub fn spawn_refresh_if_needed(&mut self) {
        for tab in self.tiles.visible_tabs() {
            self.spawn_refresh_tab(tab);
        }
    }

    fn spawn_refresh_tab(&mut self, tab: Tab) {
        match tab {
            Tab::Weather => {
                if self.weather.needs_refresh() {
                    self.weather.set_loading();
                    let lat = self.config.weather_lat.clone();
                    let lon = self.config.weather_lon.clone();
                    let name = self.config.weather_location_name.clone();
                    let tx = self.tx.clone();
                    tokio::spawn(async move {
                        match crate::api::weather::fetch_weather(&lat, &lon, &name).await {
                            Ok(data) => { let _ = tx.send(ApiUpdate::Weather(data)); }
                            Err(e) => { let _ = tx.send(ApiUpdate::WeatherError(e.to_string())); }
                        }
                    });
                }
            }
            Tab::Rss => {
                if self.rss.needs_refresh() {
                    self.rss.set_loading();
                    let feeds: Vec<String> = self.config.rss_feeds.iter()
                        .zip(&self.rss_feed_enabled)
                        .filter_map(|(url, &on)| if on { Some(url.clone()) } else { None })
                        .collect();
                    let tx = self.tx.clone();
                    tokio::spawn(async move {
                        match crate::api::rss::fetch_feeds(&feeds).await {
                            Ok(items) => { let _ = tx.send(ApiUpdate::Rss(items)); }
                            Err(e) => { let _ = tx.send(ApiUpdate::RssError(e.to_string())); }
                        }
                    });
                }
            }
            Tab::Calendar => {
                if self.calendar.needs_refresh() {
                    self.calendar.set_loading();
                    let client_id = self.config.google_client_id.clone();
                    let client_secret = self.config.google_client_secret.clone();
                    let calendar_id = self.config.google_calendar_id.clone();
                    let tx = self.tx.clone();
                    tokio::spawn(async move {
                        crate::api::calendar::fetch_events_with_auth(
                            client_id, client_secret, calendar_id, tx,
                        )
                        .await;
                    });
                }
            }
        }
    }

    pub fn check_periodic_refresh(&mut self) {
        self.spawn_refresh_if_needed();
    }

    /// Returns true if the app should quit.
    pub fn handle_key(&mut self, key: KeyCode) -> bool {
        // ── calendar day picker modal ──────────────────────────────────────────
        if self.calendar_day_picker_open {
            match key {
                KeyCode::Char('d') | KeyCode::Esc => {
                    self.calendar_day_picker_open = false;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.calendar_day_cursor + 1 < 7 {
                        self.calendar_day_cursor += 1;
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.calendar_day_cursor > 0 {
                        self.calendar_day_cursor -= 1;
                    }
                }
                KeyCode::Char(' ') | KeyCode::Enter => {
                    let idx = self.calendar_day_cursor;
                    self.calendar_days_enabled[idx] = !self.calendar_days_enabled[idx];
                }
                _ => {}
            }
            return false;
        }

        // ── feed picker modal (captures all keys while open) ──────────────────
        if self.rss_feed_picker_open {
            match key {
                KeyCode::Char('f') | KeyCode::Esc => {
                    self.rss_feed_picker_open = false;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let max = self.config.rss_feeds.len();
                    if max > 0 && self.rss_feed_cursor + 1 < max {
                        self.rss_feed_cursor += 1;
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.rss_feed_cursor > 0 {
                        self.rss_feed_cursor -= 1;
                    }
                }
                KeyCode::Char(' ') | KeyCode::Enter => {
                    let idx = self.rss_feed_cursor;
                    if let Some(enabled) = self.rss_feed_enabled.get_mut(idx) {
                        *enabled = !*enabled;
                        // Trigger re-fetch with updated filter
                        self.rss.last_refresh = None;
                        self.rss.loading = false;
                    }
                }
                _ => {}
            }
            return false;
        }

        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.show_help {
                    self.show_help = false;
                } else {
                    return true;
                }
            }
            KeyCode::Char('?') => self.show_help = !self.show_help,

            // ── tiling ─────────────────────────────────────────────────────
            // Split horizontally (side by side)
            KeyCode::Char('|') | KeyCode::Char('\\') => {
                self.tiles.split(SplitDir::Horizontal);
            }
            // Split vertically (top / bottom)
            KeyCode::Char('-') => {
                self.tiles.split(SplitDir::Vertical);
            }
            // Close focused pane
            KeyCode::Char('x') => {
                self.tiles.close_focused();
            }
            // Navigate between panes
            KeyCode::Tab => self.tiles.focus_next(),
            KeyCode::BackTab => self.tiles.focus_prev(),

            // ── change tab in focused pane ──────────────────────────────────
            KeyCode::Char('1') => self.tiles.set_focused_tab(Tab::Weather),
            KeyCode::Char('2') => self.tiles.set_focused_tab(Tab::Calendar),
            KeyCode::Char('3') => self.tiles.set_focused_tab(Tab::Rss),
            KeyCode::Left => {
                let idx = self.focused_tab().index();
                let n = Tab::ALL.len();
                self.tiles.set_focused_tab(Tab::from_index((idx + n - 1) % n));
            }
            KeyCode::Right => {
                self.tiles.set_focused_tab(Tab::from_index(self.focused_tab().index() + 1));
            }

            // ── force refresh focused pane ──────────────────────────────────
            KeyCode::Char('r') => self.force_refresh_focused(),

            // ── RSS feed picker ─────────────────────────────────────────────
            KeyCode::Char('f') if self.focused_tab() == Tab::Rss => {
                self.rss_feed_picker_open = true;
                self.rss_feed_cursor = 0;
            }

            // ── Calendar day picker ─────────────────────────────────────────
            KeyCode::Char('d') if self.focused_tab() == Tab::Calendar => {
                self.calendar_day_picker_open = true;
                self.calendar_day_cursor = 0;
            }

            // ── per-tab scroll ──────────────────────────────────────────────
            KeyCode::Enter => {
                if self.focused_tab() == Tab::Rss {
                    if let Some(items) = &self.rss.data {
                        if let Some(link) = items.get(self.rss_selected).and_then(|i| i.link.as_deref()) {
                            let _ = open::that(link);
                        }
                    }
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                match self.focused_tab() {
                    Tab::Rss => {
                        let max = self.rss.data.as_ref().map(|d| d.len()).unwrap_or(0);
                        if max > 0 && self.rss_selected + 1 < max {
                            self.rss_selected += 1;
                        }
                    }
                    Tab::Calendar => {
                        self.calendar_scroll = self.calendar_scroll.saturating_add(1);
                    }
                    _ => {}
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                match self.focused_tab() {
                    Tab::Rss => {
                        if self.rss_selected > 0 {
                            self.rss_selected -= 1;
                        }
                    }
                    Tab::Calendar => {
                        self.calendar_scroll = self.calendar_scroll.saturating_sub(1);
                    }
                    _ => {}
                }
            }

            _ => {}
        }
        false
    }

    fn force_refresh_focused(&mut self) {
        match self.focused_tab() {
            Tab::Weather => {
                self.weather.last_refresh = None;
                self.weather.loading = false;
            }
            Tab::Rss => {
                self.rss.last_refresh = None;
                self.rss.loading = false;
            }
            Tab::Calendar => {
                self.calendar.last_refresh = None;
                self.calendar.loading = false;
                self.calendar_scroll = 0;
            }
        }
    }
}
