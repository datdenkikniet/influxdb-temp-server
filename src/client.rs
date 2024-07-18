use std::time::Duration;

use chrono::{DateTime, FixedOffset};
use influxdb2::{models::Query, FromDataPoint};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, FromDataPoint, Default)]
pub struct DataPointWithOffset {
    pub time: DateTime<FixedOffset>,
    pub temperature: f64,
    pub humidity: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Humidity {
    pub value: f64,
    pub time: i64,
}

impl From<DataPointWithOffset> for Humidity {
    fn from(value: DataPointWithOffset) -> Self {
        Self {
            value: (value.humidity * 100.).round() / 100.,
            time: value.time.timestamp_millis(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Temperature {
    pub value: f64,
    pub time: i64,
}

impl From<DataPointWithOffset> for Temperature {
    fn from(value: DataPointWithOffset) -> Self {
        Self {
            value: (value.temperature * 100.).round() / 100.,
            time: value.time.timestamp_millis(),
        }
    }
}

macro_rules! log_err {
    ($thing:expr) => {
        match $thing {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("{e:?}");
                None
            }
        }
    };
}

pub struct Client {
    inner: influxdb2::Client,
}

impl Client {
    pub fn new(inner: influxdb2::Client) -> Self {
        Self { inner }
    }

    async fn in_range<O: From<DataPointWithOffset>>(
        &mut self,
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
        let res: Vec<DataPointWithOffset> = self
            .inner
            .query(Some(query))
            .await
            .map_err(|e| format!("{e}"))?;

        Ok(res.into_iter().map(O::from))
    }

    pub async fn get_temps_from_to(
        &mut self,
        start_ms: u64,
        stop_ms: u64,
    ) -> Result<impl Iterator<Item = Temperature>, String> {
        let duration_ms = stop_ms - start_ms;
        let window = 30000.max(duration_ms / 1000);

        let start = start_ms / 1000;
        let stop = (stop_ms + 1000) / 1000;

        self.in_range(&format!("start: {start}, stop: {stop}"), window)
            .await
    }

    pub async fn get_temps_in_span(
        &mut self,
        duration: Duration,
    ) -> Result<impl Iterator<Item = Temperature>, String> {
        let duration_ms = duration.as_millis();
        let window = 30000.max(duration_ms / 1000);

        self.in_range(&format!("start: -{duration_ms}ms"), window as u64)
            .await
    }

    pub async fn get_hums_from_to(
        &mut self,
        start_ms: u64,
        stop_ms: u64,
    ) -> Result<impl Iterator<Item = Humidity>, String> {
        let duration_ms = stop_ms - start_ms;
        let window = 30000.max(duration_ms / 1000);

        let start = start_ms / 1000;
        let stop = (stop_ms + 1000) / 1000;

        self.in_range(&format!("start: {start}, stop: {stop}"), window)
            .await
    }

    pub async fn get_hums_in_span(
        &mut self,
        duration: Duration,
    ) -> Result<impl Iterator<Item = Humidity>, String> {
        let duration_ms = duration.as_millis();
        let window = 30000.max(duration_ms / 1000);

        self.in_range(&format!("start: -{duration_ms}ms"), window as u64)
            .await
    }

    pub async fn get_current_temp(&mut self) -> Option<Temperature> {
        let query = format!(
            r#"
        from(bucket: "Temperature")
            |> range(start: -1d)
            |> filter(fn: (r) => r["_measurement"]  == "aht10")
            |> filter(fn: (r) => r["_field"] == "temperature")
            |> last()"#,
        );

        let query = Query::new(query.to_string());
        let res: Vec<DataPointWithOffset> = log_err!(self.inner.query(Some(query)).await)?;

        res.into_iter().map(Temperature::from).next()
    }
}
