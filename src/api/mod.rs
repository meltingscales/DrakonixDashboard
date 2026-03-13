pub mod calendar;
pub mod rss;
pub mod weather;

#[derive(Debug)]
pub enum ApiUpdate {
    Weather(weather::WeatherData),
    Rss(Vec<rss::RssItem>),
    Calendar(Vec<calendar::CalendarEvent>),
    /// Sent while waiting for the user to authorize in their browser.
    CalendarNeedAuth(String),
    WeatherError(String),
    RssError(String),
    CalendarError(String),
}
