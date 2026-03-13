use std::sync::mpsc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::ApiUpdate;

// ── public types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CalendarEvent {
    pub title: String,
    pub start: String,
    pub end: String,
    pub location: Option<String>,
}

// ── token storage ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct StoredTokens {
    access_token: String,
    refresh_token: String,
    expires_at: i64, // unix seconds
}

fn token_path() -> std::path::PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join(".drakonix_gcal.json")
}

fn load_tokens() -> Result<StoredTokens> {
    let data = std::fs::read_to_string(token_path())?;
    Ok(serde_json::from_str(&data)?)
}

fn save_tokens(t: &StoredTokens) -> Result<()> {
    std::fs::write(token_path(), serde_json::to_string(t)?)?;
    Ok(())
}

// ── OAuth helpers ─────────────────────────────────────────────────────────────

fn find_free_port() -> Result<u16> {
    let sock = std::net::TcpListener::bind("127.0.0.1:0")?;
    Ok(sock.local_addr()?.port())
}

fn random_state() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    format!("{ns:08x}")
}

fn build_auth_url(client_id: &str, redirect_uri: &str, state: &str) -> String {
    format!(
        "https://accounts.google.com/o/oauth2/v2/auth\
         ?client_id={}&redirect_uri={}&response_type=code\
         &scope={}&state={}&access_type=offline&prompt=consent",
        urlencoding::encode(client_id),
        urlencoding::encode(redirect_uri),
        urlencoding::encode("https://www.googleapis.com/auth/calendar.readonly"),
        urlencoding::encode(state),
    )
}

/// Spin up a one-shot local HTTP server, return the `code` from Google's redirect.
async fn wait_for_callback(port: u16, expected_state: &str) -> Result<String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener =
        tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;

    let (mut stream, _) =
        tokio::time::timeout(Duration::from_secs(300), listener.accept())
            .await
            .map_err(|_| anyhow!("Timed out waiting for browser authorization (5 min)"))?
            ?;

    let mut buf = vec![0u8; 8192];
    let n = stream.read(&mut buf).await?;
    let request = String::from_utf8_lossy(&buf[..n]);

    let ok_html = b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
        <html><body><h2>Authorization complete!</h2>\
        <p>Return to DrakonixDashboard.</p></body></html>";
    let err_html = b"HTTP/1.1 400 Bad Request\r\n\r\nAuthorization failed.";

    let code = match extract_query_param(&request, "code") {
        Some(c) => c,
        None => {
            let _ = stream.write_all(err_html).await;
            return Err(anyhow!("No authorization code received from Google"));
        }
    };

    let state = extract_query_param(&request, "state").unwrap_or_default();
    if state != expected_state {
        let _ = stream.write_all(err_html).await;
        return Err(anyhow!("OAuth state mismatch (possible CSRF)"));
    }

    let _ = stream.write_all(ok_html).await;
    Ok(code)
}

fn extract_query_param(request: &str, name: &str) -> Option<String> {
    let prefix = format!("{name}=");
    let first_line = request.lines().next()?;          // GET /?code=X&state=Y HTTP/1.1
    let query = first_line.split('?').nth(1)?.split(' ').next()?;
    query
        .split('&')
        .find(|p| p.starts_with(&prefix))?
        .strip_prefix(&prefix)
        .map(|v| urlencoding::decode(v).unwrap_or_default().into_owned())
}

// ── token exchange / refresh ──────────────────────────────────────────────────

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
}

async fn exchange_code(
    http: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    code: &str,
) -> Result<StoredTokens> {
    let resp = http
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", code),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Token exchange failed {status}: {body}"));
    }

    let t: TokenResponse = resp.json().await?;
    let expires_at = Utc::now().timestamp() + t.expires_in.unwrap_or(3600) as i64;
    Ok(StoredTokens {
        access_token: t.access_token,
        refresh_token: t.refresh_token.unwrap_or_default(),
        expires_at,
    })
}

async fn refresh_access_token(
    http: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<(String, i64)> {
    let resp = http
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("refresh_token", refresh_token),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Token refresh failed {status}: {body}"));
    }

    let t: TokenResponse = resp.json().await?;
    let expires_at = Utc::now().timestamp() + t.expires_in.unwrap_or(3600) as i64;
    Ok((t.access_token, expires_at))
}

// ── main auth flow ────────────────────────────────────────────────────────────

/// Returns a valid access token, running the full browser OAuth flow if needed.
async fn get_valid_access_token(
    http: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    tx: &mpsc::SyncSender<ApiUpdate>,
) -> Result<String> {
    // 1. Try stored tokens
    if let Ok(stored) = load_tokens() {
        let now = Utc::now().timestamp();
        if stored.expires_at > now + 60 {
            return Ok(stored.access_token);
        }
        // Try refreshing
        if !stored.refresh_token.is_empty() {
            if let Ok((access, expires_at)) =
                refresh_access_token(http, client_id, client_secret, &stored.refresh_token).await
            {
                let updated = StoredTokens {
                    access_token: access.clone(),
                    refresh_token: stored.refresh_token,
                    expires_at,
                };
                let _ = save_tokens(&updated);
                return Ok(access);
            }
            // Refresh failed — fall through to re-auth
        }
    }

    // 2. Full browser OAuth flow
    let port = find_free_port()?;
    let redirect_uri = format!("http://127.0.0.1:{port}");
    let state = random_state();
    let auth_url = build_auth_url(client_id, &redirect_uri, &state);

    // Notify UI (shows URL as fallback) and open browser
    let _ = tx.send(ApiUpdate::CalendarNeedAuth(auth_url.clone()));
    let _ = open::that(&auth_url);

    let code = wait_for_callback(port, &state).await?;
    let stored = exchange_code(http, client_id, client_secret, &redirect_uri, &code).await?;
    save_tokens(&stored)?;

    Ok(stored.access_token)
}

// ── calendar fetch ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct EventList {
    items: Vec<GCalEvent>,
}

#[derive(Deserialize)]
struct GCalEvent {
    summary: Option<String>,
    start: EventTime,
    end: EventTime,
    location: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventTime {
    date_time: Option<String>,
    date: Option<String>,
}

impl EventTime {
    fn display(&self) -> String {
        if let Some(dt) = &self.date_time {
            dt.get(..16)
                .map(|s| s.replace('T', " "))
                .unwrap_or_else(|| dt.clone())
        } else {
            self.date.clone().unwrap_or_default()
        }
    }
}

async fn do_fetch_events(
    http: &reqwest::Client,
    access_token: &str,
    calendar_id: &str,
) -> Result<Vec<CalendarEvent>> {
    let now = Utc::now().to_rfc3339();
    let url = format!(
        "https://www.googleapis.com/calendar/v3/calendars/{}/events\
         ?timeMin={}&maxResults=20&singleEvents=true&orderBy=startTime",
        urlencoding::encode(calendar_id),
        urlencoding::encode(&now),
    );

    let resp = http.get(&url).bearer_auth(access_token).send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Calendar API {status}: {body}"));
    }

    let list: EventList = resp.json().await?;
    Ok(list
        .items
        .into_iter()
        .map(|e| CalendarEvent {
            title: e.summary.unwrap_or_else(|| "(no title)".to_string()),
            start: e.start.display(),
            end: e.end.display(),
            location: e.location,
        })
        .collect())
}

// ── public entry point ────────────────────────────────────────────────────────

pub async fn fetch_events_with_auth(
    client_id: String,
    client_secret: String,
    calendar_id: String,
    tx: mpsc::SyncSender<ApiUpdate>,
) {
    if client_id.is_empty() || client_secret.is_empty() {
        let _ = tx.send(ApiUpdate::CalendarError(
            "GOOGLE_CLIENT_ID / GOOGLE_CLIENT_SECRET not set in .env".to_string(),
        ));
        return;
    }

    let http = reqwest::Client::new();

    let access_token =
        match get_valid_access_token(&http, &client_id, &client_secret, &tx).await {
            Ok(t) => t,
            Err(e) => {
                let _ = tx.send(ApiUpdate::CalendarError(e.to_string()));
                return;
            }
        };

    match do_fetch_events(&http, &access_token, &calendar_id).await {
        Ok(events) => {
            let _ = tx.send(ApiUpdate::Calendar(events));
        }
        Err(e) => {
            let _ = tx.send(ApiUpdate::CalendarError(e.to_string()));
        }
    }
}
