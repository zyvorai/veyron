#[cfg(feature = "web")]
use axum::{Router, response::IntoResponse, routing::get};
use serde::{Deserialize, Serialize};
#[cfg(feature = "web")]
use super::not_implemented;

/// Forecast prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastPrediction {
    pub metric: String,
    pub current_value: f64,
    pub predicted_value: f64,
    pub confidence: f64,
    pub prediction_window: String,
    pub trend: String,
    pub alert_threshold: Option<f64>,
    pub estimated_breach: Option<String>,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new().route("/forecasting/predictions", get(list_predictions))
}

#[cfg(feature = "web")]
async fn list_predictions() -> impl IntoResponse {
    not_implemented("Forecasting predictions")
}
