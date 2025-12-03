use crate::config::Config;
use crate::state::WeatherInfo;
use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct OpenWeatherResponse {
    main: OpenWeatherMain,
    weather: Vec<OpenWeatherWeather>,
    name: String,
    sys: OpenWeatherSys,
}

#[derive(Debug, Deserialize)]
struct OpenWeatherMain {
    temp: f64,
    #[allow(dead_code)]
    feels_like: f64,
    #[allow(dead_code)]
    humidity: i32,
}

#[derive(Debug, Deserialize)]
struct OpenWeatherWeather {
    #[allow(dead_code)]
    id: i32,
    #[allow(dead_code)]
    main: String,
    description: String,
    icon: String,
}

#[derive(Debug, Deserialize)]
struct OpenWeatherSys {
    country: String,
}

pub struct WeatherClient {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl WeatherClient {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
        }
    }

    pub async fn fetch_weather(&self, config: &Config) -> Result<WeatherInfo> {
        if let Some(ref api_key) = self.api_key {
            self.fetch_openweather(config, api_key).await
        } else {
            Ok(self.get_fallback_weather(config))
        }
    }

    async fn fetch_openweather(&self, config: &Config, api_key: &str) -> Result<WeatherInfo> {
        let url = format!(
            "https://api.openweathermap.org/data/2.5/weather?q={}&appid={}&units=metric",
            urlencoding::encode(&config.weather.location),
            api_key
        );

        let response = match self.client.get(&url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                tracing::warn!("Failed to connect to weather service: {}", e);
                return Ok(self.fallback_weather(config, "Network error"));
            }
        };

        // Handle specific HTTP status codes
        let status = response.status();
        if !status.is_success() {
            match status.as_u16() {
                401 => {
                    tracing::warn!("Weather API key is invalid or missing");
                    return Ok(self.fallback_weather(config, "Invalid API key"));
                }
                403 => {
                    tracing::warn!("Weather API access forbidden - check API key permissions");
                    return Ok(self.fallback_weather(config, "API access forbidden"));
                }
                429 => {
                    tracing::warn!("Weather API rate limit exceeded");
                    return Ok(self.fallback_weather(config, "Rate limit exceeded"));
                }
                404 => {
                    tracing::warn!("Weather location '{}' not found", config.weather.location);
                    return Ok(self.fallback_weather(config, "Location not found"));
                }
                500..=599 => {
                    tracing::warn!("Weather service is currently unavailable ({})", status);
                    return Ok(self.fallback_weather(config, "Service unavailable"));
                }
                _ => {
                    tracing::warn!("Weather API returned unexpected status: {}", status);
                    return Ok(self.fallback_weather(config, "API error"));
                }
            }
        }

        let weather_data: OpenWeatherResponse = match response.json().await {
            Ok(data) => data,
            Err(e) => {
                tracing::warn!("Failed to parse weather response: {}", e);
                return Ok(self.fallback_weather(config, "Invalid response"));
            }
        };

        let primary_weather = match weather_data.weather.first() {
            Some(weather) => weather,
            None => {
                tracing::warn!("Weather response contained no weather data");
                return Ok(self.fallback_weather(config, "No weather data"));
            }
        };

        let location_display = format!("{}, {}", weather_data.name, weather_data.sys.country);
        let temperature_c = weather_data.main.temp.round() as i32;
        let condition = primary_weather.description.clone();
        let icon_name = self.map_weather_icon(&primary_weather.icon);

        Ok(WeatherInfo {
            location_display,
            temperature_c,
            condition,
            icon_name: Some(icon_name),
        })
    }

    fn get_fallback_weather(&self, config: &Config) -> WeatherInfo {
        WeatherInfo {
            location_display: config.weather.location.clone(),
            temperature_c: 20,
            condition: "Weather data unavailable".to_string(),
            icon_name: Some("unknown".to_string()),
        }
    }

    fn fallback_weather(&self, config: &Config, reason: &str) -> WeatherInfo {
        WeatherInfo {
            location_display: config.weather.location.clone(),
            temperature_c: 20,
            condition: format!("Weather unavailable: {}", reason),
            icon_name: Some("unknown".to_string()),
        }
    }

    fn map_weather_icon(&self, openweather_icon: &str) -> String {
        let icon_map = self.get_icon_mapping();
        
        // Remove day/night indicator (last character) to get base icon
        let base_icon = &openweather_icon[..openweather_icon.len()-1];
        
        icon_map.get(base_icon)
            .or_else(|| icon_map.get(openweather_icon))
            .unwrap_or(&"unknown".to_string())
            .clone()
    }

    fn get_icon_mapping(&self) -> HashMap<&'static str, String> {
        let mut map = HashMap::new();
        
        // Clear sky
        map.insert("01", "clear".to_string());
        
        // Few clouds
        map.insert("02", "few-clouds".to_string());
        
        // Scattered clouds
        map.insert("03", "scattered-clouds".to_string());
        
        // Broken clouds
        map.insert("04", "broken-clouds".to_string());
        
        // Shower rain
        map.insert("09", "shower-rain".to_string());
        
        // Rain
        map.insert("10", "rain".to_string());
        
        // Thunderstorm
        map.insert("11", "thunderstorm".to_string());
        
        // Snow
        map.insert("13", "snow".to_string());
        
        // Mist/fog
        map.insert("50", "mist".to_string());
        
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[tokio::test]
    async fn test_fallback_weather() {
        let client = WeatherClient::new(None);
        let config = Config::default();
        
        let weather = client.fetch_weather(&config).await.unwrap();
        
        assert_eq!(weather.location_display, config.weather.location);
        assert_eq!(weather.temperature_c, 20);
        assert_eq!(weather.condition, "Weather data unavailable");
    }

    #[test]
    fn test_icon_mapping() {
        let client = WeatherClient::new(None);
        
        assert_eq!(client.map_weather_icon("01d"), "clear");
        assert_eq!(client.map_weather_icon("01n"), "clear");
        assert_eq!(client.map_weather_icon("10d"), "rain");
        assert_eq!(client.map_weather_icon("11n"), "thunderstorm");
        assert_eq!(client.map_weather_icon("unknown"), "unknown");
    }
}