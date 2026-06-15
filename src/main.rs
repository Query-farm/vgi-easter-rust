// Copyright 2026 Query Farm LLC - https://query.farm

//! VGI worker that computes the date of Easter Sunday for a given year.
//!
//! Provides a single scalar function, `easter_date(year)`, returning the
//! Gregorian (Western) date of Easter Sunday using the Anonymous Gregorian
//! algorithm (a.k.a. the Meeus/Jones/Butcher *Computus*). This is a faithful
//! Rust port of the Python `vgi-easter` worker.
//!
//! Usage:
//! ```text
//!   vgi-easter            # stdio transport (DuckDB spawns it)
//!   vgi-easter --http     # HTTP transport (serves on :8080)
//!
//!   SELECT easter_date(2025);
//!   SELECT year, easter_date(year) AS easter FROM range(2020, 2031) t(year);
//! ```

mod easter;

use arrow_array::cast::AsArray;
use arrow_array::{Array, Date32Array, RecordBatch};
use arrow_schema::DataType;
use vgi::catalog::{CatSchema, CatalogModel};
use vgi::function::{ArgSpec, BindParams, BindResponse, FunctionMetadata, ProcessParams, ScalarFunction};
use vgi::Worker;
use vgi_rpc::{Result, RpcError};

/// The catalog's stable data version.
const DATA_VERSION: &str = "1.0.0";

/// Version reported as the catalog's `implementation_version`.
///
/// Prefer an explicit git SHA from `VGI_EASTER_GIT_COMMIT` (handy in CI/dev
/// builds); otherwise fall back to the compiled-in package version, so a normal
/// install reports the release version (e.g. `1.0.0`). `"unknown"` only if
/// neither is available.
fn implementation_version() -> String {
    if let Ok(sha) = std::env::var("VGI_EASTER_GIT_COMMIT") {
        if !sha.is_empty() {
            return sha;
        }
    }
    match option_env!("CARGO_PKG_VERSION") {
        Some(v) if !v.is_empty() => v.to_string(),
        _ => "unknown".to_string(),
    }
}

/// `easter_date(year)` — date of Western (Gregorian) Easter Sunday.
///
/// - a single `int64` column input (`year`)
/// - an explicit `date32` output type (days since the Unix epoch)
/// - null propagation (a null year yields a null date)
///
/// Examples:
/// - `SELECT easter_date(2025)` — Easter Sunday in 2025 (2025-04-20)
/// - `SELECT year, easter_date(year) AS easter FROM range(2020, 2031) t(year)`
///   — Easter dates for 2020 through 2030
pub struct EasterDateFunction;

impl ScalarFunction for EasterDateFunction {
    fn name(&self) -> &str {
        "easter_date"
    }

    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            description: "Date of Western (Gregorian) Easter Sunday for a given year".to_string(),
            return_type: Some(DataType::Date32),
            ..Default::default()
        }
    }

    fn argument_specs(&self) -> Vec<ArgSpec> {
        vec![ArgSpec::column(
            "year",
            0,
            "int64",
            "Year, e.g. 2025 (Gregorian, >= 1583)",
        )]
    }

    fn on_bind(&self, _params: &BindParams) -> Result<BindResponse> {
        Ok(BindResponse::result(DataType::Date32))
    }

    fn process(&self, params: &ProcessParams, batch: &RecordBatch) -> Result<RecordBatch> {
        let years = batch.column(0).as_primitive::<arrow_array::types::Int64Type>();
        let out: Date32Array = (0..years.len())
            .map(|i| (!years.is_null(i)).then(|| easter::easter_date32(years.value(i))))
            .collect();
        RecordBatch::try_new(params.output_schema.clone(), vec![std::sync::Arc::new(out)])
            .map_err(|e| RpcError::runtime_error(format!("build result batch: {e}")))
    }
}

/// Build the `easter` catalog model.
fn build_catalog() -> CatalogModel {
    CatalogModel {
        name: "easter".to_string(),
        implementation_version: Some(implementation_version()),
        data_version_spec: Some(DATA_VERSION.to_string()),
        comment: None,
        schemas: vec![CatSchema {
            name: "main".to_string(),
            comment: Some("Computus: the date of Western (Gregorian) Easter Sunday".to_string()),
            views: Vec::new(),
            macros: Vec::new(),
            tables: Vec::new(),
        }],
        ..Default::default()
    }
}

fn main() {
    // Logs go to stderr — stdout is the Arrow-IPC channel.
    let _ = env_logger::Builder::from_env(env_logger::Env::default().filter_or("VGI_LOG", "info"))
        .format_timestamp_millis()
        .try_init();

    let mut worker = Worker::new();
    worker.register_scalar(EasterDateFunction);
    worker.set_catalog(build_catalog());
    worker.run();
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_array::Int64Array;
    use std::sync::Arc;

    /// Run `EasterDateFunction::process` over a batch of years and return the
    /// resulting Date32Array.
    fn compute(years: Int64Array) -> Date32Array {
        let func = EasterDateFunction;
        let schema = Arc::new(arrow_schema::Schema::new(vec![arrow_schema::Field::new(
            "year",
            DataType::Int64,
            true,
        )]));
        let batch = RecordBatch::try_new(schema, vec![Arc::new(years)]).unwrap();
        let out_schema = Arc::new(arrow_schema::Schema::new(vec![arrow_schema::Field::new(
            "result",
            DataType::Date32,
            true,
        )]));
        let params = ProcessParams {
            output_schema: out_schema,
            input_schema: None,
            execution_id: Vec::new(),
            init_opaque_data: Vec::new(),
            arguments: Default::default(),
            settings: Default::default(),
            secrets: Default::default(),
            auth_principal: None,
            projection_ids: None,
            pushdown_filters: None,
            join_keys: Vec::new(),
            storage: None,
            order_by_column: None,
            order_by_direction: None,
            order_by_null_order: None,
            order_by_limit: None,
            tablesample_percentage: None,
            tablesample_seed: None,
            attach_opaque_data: None,
            at_unit: None,
            at_value: None,
        };
        let result = func.process(&params, &batch).unwrap();
        result
            .column(0)
            .as_primitive::<arrow_array::types::Date32Type>()
            .clone()
    }

    fn d(year: i32, month: u32, day: u32) -> i32 {
        easter::days_from_civil(easter::CivilDate { year, month, day })
    }

    #[test]
    fn test_compute_batch_returns_date32() {
        let result = compute(Int64Array::from(vec![2024, 2025, 2026]));
        assert_eq!(result.data_type(), &DataType::Date32);
        assert_eq!(result.value(0), d(2024, 3, 31));
        assert_eq!(result.value(1), d(2025, 4, 20));
        assert_eq!(result.value(2), d(2026, 4, 5));
        assert!(!result.is_null(0));
    }

    #[test]
    fn test_compute_null_propagation() {
        let result = compute(Int64Array::from(vec![Some(2025), None, Some(2026)]));
        assert_eq!(result.value(0), d(2025, 4, 20));
        assert!(result.is_null(1));
        assert_eq!(result.value(2), d(2026, 4, 5));
    }

    #[test]
    fn test_compute_extremes() {
        let result = compute(Int64Array::from(vec![1818, 1943]));
        assert_eq!(result.value(0), d(1818, 3, 22)); // earliest Easter
        assert_eq!(result.value(1), d(1943, 4, 25)); // latest Easter
    }

    // Both halves of version resolution share the process-wide
    // VGI_EASTER_GIT_COMMIT env var, so they live in one test to avoid a data
    // race with Rust's parallel test runner.
    #[test]
    fn test_implementation_version_resolution() {
        // Prefers the git SHA when set.
        std::env::set_var("VGI_EASTER_GIT_COMMIT", "deadbeef");
        assert_eq!(implementation_version(), "deadbeef");
        // Falls back to the compiled-in package version otherwise.
        std::env::remove_var("VGI_EASTER_GIT_COMMIT");
        assert_eq!(implementation_version(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_catalog_shape() {
        let cat = build_catalog();
        assert_eq!(cat.name, "easter");
        assert_eq!(cat.data_version_spec.as_deref(), Some("1.0.0"));
        assert_eq!(cat.schemas.len(), 1);
        assert_eq!(cat.schemas[0].name, "main");
        assert_eq!(
            cat.schemas[0].comment.as_deref(),
            Some("Computus: the date of Western (Gregorian) Easter Sunday")
        );
    }
}
