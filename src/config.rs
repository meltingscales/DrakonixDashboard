pub struct Config {
    pub weather_lat: String,
    pub weather_lon: String,
    pub weather_location_name: String,
    pub google_client_id: String,
    pub google_client_secret: String,
    pub google_calendar_id: String,
    pub rss_feeds: Vec<String>,
}

impl Config {
    pub fn from_env() -> Self {
        // Client ID/secret: prefer explicit env vars, fall back to credentials.json
        let (json_id, json_secret) = credentials_from_json();
        Config {
            weather_lat: std::env::var("WEATHER_LAT").unwrap_or_else(|_| "40.71".to_string()),
            weather_lon: std::env::var("WEATHER_LON").unwrap_or_else(|_| "-74.01".to_string()),
            weather_location_name: std::env::var("WEATHER_LOCATION_NAME").unwrap_or_default(),
            google_client_id: nonempty_env("GOOGLE_CLIENT_ID").unwrap_or(json_id),
            google_client_secret: nonempty_env("GOOGLE_CLIENT_SECRET").unwrap_or(json_secret),
            google_calendar_id: std::env::var("GOOGLE_CALENDAR_ID")
                .unwrap_or_else(|_| "primary".to_string()),
            rss_feeds: std::env::var("RSS_FEEDS")
                .unwrap_or_default()
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect(),
        }
    }
}

/// Returns `Some(value)` only if the env var is set AND non-empty.
fn nonempty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

/// Read client_id and client_secret from the file at GOOGLE_CREDENTIALS_JSON (if set).
/// The file is the `credentials.json` downloaded from Google Cloud Console.
fn credentials_from_json() -> (String, String) {
    let path = match std::env::var("GOOGLE_CREDENTIALS_JSON") {
        Ok(p) => p,
        Err(_) => return (String::new(), String::new()),
    };

    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => return (String::new(), String::new()),
    };

    // credentials.json wraps everything under an "installed" or "web" key
    let v: serde_json::Value = match serde_json::from_str(&data) {
        Ok(v) => v,
        Err(_) => return (String::new(), String::new()),
    };

    let root = v.get("installed").or_else(|| v.get("web")).unwrap_or(&v);
    let id = root["client_id"].as_str().unwrap_or("").to_string();
    let secret = root["client_secret"].as_str().unwrap_or("").to_string();
    (id, secret)
}
