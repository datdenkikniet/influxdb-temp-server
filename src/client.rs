use std::time::Duration;

use chrono::{DateTime, FixedOffset};
use influxdb2::{models::Query, FromDataPoint};
use influxdb2_structmap::FromMap;
use serde::Serialize;

pub struct Client {
    inner: influxdb2::Client,
}

#[derive(Debug, Clone, FromDataPoint, Default)]
pub struct DataPointWithOffset {
    pub time: DateTime<FixedOffset>,
    pub temperature: f64,
    pub humidity: f64,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct DataPoint {
    pub time: i64,
    pub temperature: f64,
    pub humidity: f64,
}

impl From<DataPointWithOffset> for DataPoint {
    fn from(value: DataPointWithOffset) -> Self {
        Self {
            temperature: (value.temperature * 100.).round() / 100.,
            humidity: (value.humidity * 100.).round() / 100.,
            time: value.time.timestamp_millis(),
        }
    }
}

impl Client {
    pub fn new(inner: influxdb2::Client) -> Self {
        Self { inner }
    }

    async fn in_range<T: FromMap, O: From<T>>(
        &self,
        range: &str,
        window: u64,
    ) -> Result<impl Iterator<Item = O>, String> {
        let query = format!(
            r#"
        from(bucket: "Temperature")
            |> range({range})
            |> filter(fn: (r) => r["_measurement"]  == "aht10")
            |> aggregateWindow(every: {window}ms, fn: mean, createEmpty: false)
            |> yield(name: "mean")"#,
        );

        let query = Query::new(query.to_string());
        let res: Vec<T> = self
            .inner
            .query(Some(query))
            .await
            .map_err(|e| format!("{e}"))?;

        Ok(res.into_iter().map(O::from))
    }

    pub async fn get_data_in_span(
        &self,
        duration: Duration,
    ) -> Result<impl Iterator<Item = DataPoint>, String> {
        let duration_ms = duration.as_millis();
        let window = 30000.max(duration_ms / 1000);

        self.in_range::<DataPointWithOffset, _>(&format!("start: -{duration_ms}ms"), window as u64)
            .await
    }

    pub async fn get_data_from_to(
        &self,
        start_ms: u64,
        stop_ms: u64,
    ) -> Result<impl Iterator<Item = DataPoint>, String> {
        let duration_ms = stop_ms - start_ms;
        let window = 30000.max(duration_ms / 1000);

        let start = start_ms / 1000;
        let stop = (stop_ms + 1000) / 1000;

        self.in_range::<DataPointWithOffset, _>(&format!("start: {start}, stop: {stop}"), window)
            .await
    }

    pub async fn get_current(&self) -> Option<DataPoint> {
        self.get_data_in_span(Duration::from_secs(60 * 60 * 24))
            .await
            .ok()?
            .into_iter()
            .last()
    }
}
