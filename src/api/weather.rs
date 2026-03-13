use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct DayForecast {
    pub date: String,       // "Mon Jan 13"
    pub high_f: f64,
    pub low_f: f64,
    pub precip_in: f64,
    pub wind_max_mph: f64,
    pub description: String,
    pub icon: &'static str,
}

#[derive(Debug, Clone)]
pub struct WeatherData {
    pub location: String,
    // current conditions
    pub temp_f: f64,
    pub feels_like_f: f64,
    pub humidity: u64,
    pub wind_mph: f64,
    pub description: String,
    pub icon: &'static str,
    // 7-day forecast; index 0 = today
    pub forecast: Vec<DayForecast>,
}

#[derive(Deserialize)]
struct OpenMeteoResponse {
    current: Current,
    daily: Daily,
}

#[derive(Deserialize)]
struct Current {
    temperature_2m: f64,
    apparent_temperature: f64,
    relative_humidity_2m: u64,
    wind_speed_10m: f64,
    weather_code: u64,
}

#[derive(Deserialize)]
struct Daily {
    time: Vec<String>,
    temperature_2m_max: Vec<f64>,
    temperature_2m_min: Vec<f64>,
    weather_code: Vec<u64>,
    precipitation_sum: Vec<f64>,
    wind_speed_10m_max: Vec<f64>,
}

pub async fn fetch_weather(lat: &str, lon: &str, location_name: &str) -> Result<WeatherData> {
    let url = format!(
        "https://api.open-meteo.com/v1/forecast\
         ?latitude={lat}&longitude={lon}\
         &current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,weather_code\
         &daily=temperature_2m_max,temperature_2m_min,weather_code,precipitation_sum,wind_speed_10m_max\
         &temperature_unit=fahrenheit\
         &wind_speed_unit=mph\
         &precipitation_unit=inch\
         &timezone=auto\
         &forecast_days=7"
    );

    let resp: OpenMeteoResponse = reqwest::get(&url).await?.json().await?;

    let cur_code = resp.current.weather_code;
    let forecast = resp.daily.time.iter().enumerate()
        .map(|(i, date_str)| {
            let code = resp.daily.weather_code.get(i).copied().unwrap_or(0);
            let raw = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                .map(|d| d.format("%a %b %d").to_string())
                .unwrap_or_else(|_| date_str.clone());
            DayForecast {
                date: raw,
                high_f:       resp.daily.temperature_2m_max.get(i).copied().unwrap_or(0.0),
                low_f:        resp.daily.temperature_2m_min.get(i).copied().unwrap_or(0.0),
                precip_in:    resp.daily.precipitation_sum.get(i).copied().unwrap_or(0.0),
                wind_max_mph: resp.daily.wind_speed_10m_max.get(i).copied().unwrap_or(0.0),
                description:  wmo_description(code).to_string(),
                icon:         wmo_icon(code),
            }
        })
        .collect();

    Ok(WeatherData {
        location: if location_name.is_empty() {
            format!("{lat}, {lon}")
        } else {
            location_name.to_string()
        },
        temp_f:       resp.current.temperature_2m,
        feels_like_f: resp.current.apparent_temperature,
        humidity:     resp.current.relative_humidity_2m,
        wind_mph:     resp.current.wind_speed_10m,
        description:  wmo_description(cur_code).to_string(),
        icon:         wmo_icon(cur_code),
        forecast,
    })
}

fn wmo_description(code: u64) -> &'static str {
    match code {
        0        => "Clear sky",
        1        => "Mainly clear",
        2        => "Partly cloudy",
        3        => "Overcast",
        45 | 48  => "Fog",
        51|53|55 => "Drizzle",
        56 | 57  => "Freezing drizzle",
        61|63|65 => "Rain",
        66 | 67  => "Freezing rain",
        71|73|75 => "Snow",
        77       => "Snow grains",
        80|81|82 => "Showers",
        85 | 86  => "Snow showers",
        95       => "Thunderstorm",
        96 | 99  => "Tstorm + hail",
        _        => "Unknown",
    }
}

fn wmo_icon(code: u64) -> &'static str {
    match code {
        0        => "☀",
        1 | 2    => "⛅",
        3        => "☁",
        45 | 48  => "🌫",
        51..=57 | 80..=82 => "🌦",
        61..=67  => "🌧",
        71..=77 | 85 | 86 => "❄",
        95..=99  => "⛈",
        _        => "?",
    }
}
