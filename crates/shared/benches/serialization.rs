use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use serde::{Deserialize, Serialize};

// Representative types matching the crate's data patterns.
// These mirror the actual structs flowing through Redis commands and API calls.

/// Small struct - similar to redis::types::Point (2 fields)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeoPoint {
    lat: f64,
    lon: f64,
}

/// Medium struct - similar to callapi::ErrorBody (conditional serialization)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiError {
    #[serde(skip_serializing_if = "String::is_empty")]
    error_message: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    error_code: String,
}

/// Large struct - similar to redis::types::RedisSettings (13 fields)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServiceConfig {
    host: String,
    port: u16,
    cluster_enabled: bool,
    cluster_urls: Vec<String>,
    use_legacy_version: bool,
    pool_size: usize,
    reconnect_max_attempts: u32,
    reconnect_delay: u32,
    default_ttl: u32,
    default_hash_ttl: u32,
    stream_read_count: u64,
    partition: usize,
    broadcast_channel_capacity: usize,
}

/// Nested struct - representative domain object flowing through Redis
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DriverLocation {
    driver_id: String,
    location: GeoPoint,
    timestamp: i64,
    speed: f64,
    bearing: f64,
    accuracy: f64,
    provider: String,
}

fn sample_geo_point() -> GeoPoint {
    GeoPoint {
        lat: 12.9716,
        lon: 77.5946,
    }
}

fn sample_api_error() -> ApiError {
    ApiError {
        error_message: "Internal Server Error".to_string(),
        error_code: "INTERNAL_ERROR".to_string(),
    }
}

fn sample_service_config() -> ServiceConfig {
    ServiceConfig {
        host: "redis.example.com".to_string(),
        port: 6379,
        cluster_enabled: true,
        cluster_urls: vec![
            "node1.redis.example.com:6380".to_string(),
            "node2.redis.example.com:6381".to_string(),
            "node3.redis.example.com:6382".to_string(),
        ],
        use_legacy_version: false,
        pool_size: 10,
        reconnect_max_attempts: 5,
        reconnect_delay: 1000,
        default_ttl: 3600,
        default_hash_ttl: 7200,
        stream_read_count: 100,
        partition: 0,
        broadcast_channel_capacity: 32,
    }
}

fn sample_driver_location() -> DriverLocation {
    DriverLocation {
        driver_id: "driver-550e8400-e29b-41d4-a716-446655440000".to_string(),
        location: GeoPoint {
            lat: 12.9716,
            lon: 77.5946,
        },
        timestamp: 1700000000,
        speed: 45.5,
        bearing: 180.0,
        accuracy: 5.0,
        provider: "gps".to_string(),
    }
}

fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialize");

    let point = sample_geo_point();
    group.bench_function("geo_point", |b| {
        b.iter(|| serde_json::to_string(black_box(&point)))
    });

    let error = sample_api_error();
    group.bench_function("api_error", |b| {
        b.iter(|| serde_json::to_string(black_box(&error)))
    });

    // ApiError with empty fields triggers skip_serializing_if
    let empty_error = ApiError {
        error_message: String::new(),
        error_code: String::new(),
    };
    group.bench_function("api_error_empty_fields", |b| {
        b.iter(|| serde_json::to_string(black_box(&empty_error)))
    });

    let config = sample_service_config();
    group.bench_function("service_config", |b| {
        b.iter(|| serde_json::to_string(black_box(&config)))
    });

    let location = sample_driver_location();
    group.bench_function("driver_location_nested", |b| {
        b.iter(|| serde_json::to_string(black_box(&location)))
    });

    group.finish();
}

fn bench_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("deserialize");

    let point_json = serde_json::to_string(&sample_geo_point()).unwrap();
    group.bench_function("geo_point", |b| {
        b.iter(|| serde_json::from_str::<GeoPoint>(black_box(&point_json)))
    });

    let error_json = serde_json::to_string(&sample_api_error()).unwrap();
    group.bench_function("api_error", |b| {
        b.iter(|| serde_json::from_str::<ApiError>(black_box(&error_json)))
    });

    let config_json = serde_json::to_string(&sample_service_config()).unwrap();
    group.bench_function("service_config", |b| {
        b.iter(|| serde_json::from_str::<ServiceConfig>(black_box(&config_json)))
    });

    let location_json = serde_json::to_string(&sample_driver_location()).unwrap();
    group.bench_function("driver_location_nested", |b| {
        b.iter(|| serde_json::from_str::<DriverLocation>(black_box(&location_json)))
    });

    group.finish();
}

fn bench_batch_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_deserialize");

    // Simulates the mget_keys pattern where multiple JSON strings are deserialized
    for count in [10, 50, 100] {
        let json_strings: Vec<String> = (0..count)
            .map(|i| {
                serde_json::to_string(&DriverLocation {
                    driver_id: format!("driver-{i}"),
                    location: GeoPoint {
                        lat: 12.9716 + (i as f64 * 0.001),
                        lon: 77.5946 + (i as f64 * 0.001),
                    },
                    timestamp: 1700000000 + i,
                    speed: 30.0 + (i as f64),
                    bearing: (i as f64 * 10.0) % 360.0,
                    accuracy: 5.0,
                    provider: "gps".to_string(),
                })
                .unwrap()
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("driver_locations", count),
            &json_strings,
            |b, jsons| {
                b.iter(|| {
                    jsons
                        .iter()
                        .map(|s| serde_json::from_str::<DriverLocation>(black_box(s)))
                        .collect::<Result<Vec<_>, _>>()
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_serialization,
    bench_deserialization,
    bench_batch_deserialization
);
criterion_main!(benches);
