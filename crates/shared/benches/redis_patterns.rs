use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Types representing the data flowing through Redis commands.
// These mirror the actual domain objects serialized/deserialized in commands.rs.

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeoPoint {
    lat: f64,
    lon: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DriverProfile {
    id: String,
    name: String,
    phone: String,
    vehicle_number: String,
    rating: f64,
    total_rides: u64,
    active: bool,
    last_location: GeoPoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StreamEntry {
    id: String,
    timestamp: i64,
    event_type: String,
    payload: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RideEstimate {
    ride_id: String,
    distance_km: f64,
    duration_min: f64,
    fare: f64,
    surge_multiplier: f64,
    vehicle_type: String,
}

fn sample_driver_profile() -> DriverProfile {
    DriverProfile {
        id: "driver-550e8400-e29b-41d4-a716-446655440000".to_string(),
        name: "John Doe".to_string(),
        phone: "+919876543210".to_string(),
        vehicle_number: "KA01AB1234".to_string(),
        rating: 4.8,
        total_rides: 1250,
        active: true,
        last_location: GeoPoint {
            lat: 12.9716,
            lon: 77.5946,
        },
    }
}

fn sample_stream_entry() -> StreamEntry {
    let mut payload = HashMap::new();
    payload.insert("ride_id".to_string(), "ride-123".to_string());
    payload.insert("driver_id".to_string(), "driver-456".to_string());
    payload.insert("status".to_string(), "ACCEPTED".to_string());
    payload.insert("fare".to_string(), "250.00".to_string());
    StreamEntry {
        id: "1700000000-0".to_string(),
        timestamp: 1700000000,
        event_type: "ride_update".to_string(),
        payload,
    }
}

fn sample_ride_estimate() -> RideEstimate {
    RideEstimate {
        ride_id: "ride-550e8400-e29b-41d4-a716-446655440000".to_string(),
        distance_km: 12.5,
        duration_min: 25.0,
        fare: 250.0,
        surge_multiplier: 1.2,
        vehicle_type: "SEDAN".to_string(),
    }
}

/// Benchmarks the set_key serialization pipeline:
/// value -> serde_json::to_string -> String (fed into RedisValue)
fn bench_set_key_pattern(c: &mut Criterion) {
    let mut group = c.benchmark_group("redis_set_key");

    let profile = sample_driver_profile();
    group.bench_function("driver_profile", |b| {
        b.iter(|| serde_json::to_string(black_box(&profile)))
    });

    let point = GeoPoint {
        lat: 12.9716,
        lon: 77.5946,
    };
    group.bench_function("geo_point", |b| {
        b.iter(|| serde_json::to_string(black_box(&point)))
    });

    let estimate = sample_ride_estimate();
    group.bench_function("ride_estimate", |b| {
        b.iter(|| serde_json::to_string(black_box(&estimate)))
    });

    group.finish();
}

/// Benchmarks the get_key deserialization pipeline:
/// String (from RedisValue) -> serde_json::from_str -> T
fn bench_get_key_pattern(c: &mut Criterion) {
    let mut group = c.benchmark_group("redis_get_key");

    let profile_json = serde_json::to_string(&sample_driver_profile()).unwrap();
    group.bench_function("driver_profile", |b| {
        b.iter(|| serde_json::from_str::<DriverProfile>(black_box(&profile_json)))
    });

    let point_json = r#"{"lat":12.9716,"lon":77.5946}"#;
    group.bench_function("geo_point", |b| {
        b.iter(|| serde_json::from_str::<GeoPoint>(black_box(point_json)))
    });

    let estimate_json = serde_json::to_string(&sample_ride_estimate()).unwrap();
    group.bench_function("ride_estimate", |b| {
        b.iter(|| serde_json::from_str::<RideEstimate>(black_box(&estimate_json)))
    });

    group.finish();
}

/// Benchmarks hash field operations:
/// set_hash_fields serializes each field, get_all_hash_fields deserializes the map.
fn bench_hash_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("redis_hash");

    // Serializing individual hash field values (set_hash_fields pattern)
    let field_values: Vec<(&str, &str)> = vec![
        ("name", "John Doe"),
        ("phone", "+919876543210"),
        ("vehicle", "KA01AB1234"),
        ("rating", "4.8"),
        ("active", "true"),
    ];
    group.bench_function("serialize_5_fields", |b| {
        b.iter(|| {
            black_box(&field_values)
                .iter()
                .map(|(_, v)| serde_json::to_string(v).unwrap())
                .collect::<Vec<_>>()
        })
    });

    // Deserializing a HashMap response (get_all_hash_fields pattern)
    let mut hash_data = HashMap::new();
    hash_data.insert("name".to_string(), "\"John Doe\"".to_string());
    hash_data.insert("phone".to_string(), "\"+919876543210\"".to_string());
    hash_data.insert("vehicle".to_string(), "\"KA01AB1234\"".to_string());
    hash_data.insert("rating".to_string(), "4.8".to_string());
    hash_data.insert("active".to_string(), "true".to_string());
    let hash_json = serde_json::to_string(&hash_data).unwrap();

    group.bench_function("deserialize_hash_map", |b| {
        b.iter(|| serde_json::from_str::<HashMap<String, String>>(black_box(&hash_json)))
    });

    // FxHashMap construction from deserialized data (used internally)
    group.bench_function("build_fxhashmap_5_entries", |b| {
        b.iter(|| {
            let mut map = rustc_hash::FxHashMap::default();
            for (k, v) in black_box(&field_values) {
                map.insert(k.to_string(), v.to_string());
            }
            map
        })
    });

    group.finish();
}

/// Benchmarks geo operations (geo_add, geo_search serialization patterns).
fn bench_geo_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("redis_geo");

    // Batch geo point serialization (geo_add with multiple points)
    for count in [5, 20, 50] {
        let points: Vec<GeoPoint> = (0..count)
            .map(|i| GeoPoint {
                lat: 12.9 + (i as f64 * 0.01),
                lon: 77.5 + (i as f64 * 0.01),
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("serialize_batch", count),
            &points,
            |b, pts| {
                b.iter(|| {
                    pts.iter()
                        .map(|p| serde_json::to_string(black_box(p)))
                        .collect::<Result<Vec<_>, _>>()
                })
            },
        );
    }

    // Batch geo point deserialization (geo_search results)
    for count in [5, 20, 50] {
        let json_points: Vec<String> = (0..count)
            .map(|i| {
                format!(
                    r#"{{"lat":{},"lon":{}}}"#,
                    12.9 + (i as f64 * 0.01),
                    77.5 + (i as f64 * 0.01)
                )
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("deserialize_batch", count),
            &json_points,
            |b, jsons| {
                b.iter(|| {
                    jsons
                        .iter()
                        .map(|s| serde_json::from_str::<GeoPoint>(black_box(s)))
                        .collect::<Result<Vec<_>, _>>()
                })
            },
        );
    }

    group.finish();
}

/// Benchmarks stream entry operations (xadd, xread serialization patterns).
fn bench_stream_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("redis_stream");

    // Single stream entry serialization (xadd)
    let entry = sample_stream_entry();
    group.bench_function("serialize_entry", |b| {
        b.iter(|| serde_json::to_string(black_box(&entry)))
    });

    // Single stream entry deserialization (xread)
    let entry_json = serde_json::to_string(&sample_stream_entry()).unwrap();
    group.bench_function("deserialize_entry", |b| {
        b.iter(|| serde_json::from_str::<StreamEntry>(black_box(&entry_json)))
    });

    // Batch stream read (xread with multiple entries)
    for count in [10, 50] {
        let entries_json: Vec<String> = (0..count)
            .map(|i| {
                let mut payload = HashMap::new();
                payload.insert("ride_id".to_string(), format!("ride-{i}"));
                payload.insert("status".to_string(), "ACTIVE".to_string());
                serde_json::to_string(&StreamEntry {
                    id: format!("{}-0", 1700000000 + i),
                    timestamp: 1700000000 + i,
                    event_type: "ride_update".to_string(),
                    payload,
                })
                .unwrap()
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("deserialize_batch", count),
            &entries_json,
            |b, jsons| {
                b.iter(|| {
                    jsons
                        .iter()
                        .map(|s| serde_json::from_str::<StreamEntry>(black_box(s)))
                        .collect::<Result<Vec<_>, _>>()
                })
            },
        );
    }

    group.finish();
}

/// Benchmarks the sorted set value serialization pattern (zadd/zrange).
fn bench_sorted_set_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("redis_sorted_set");

    // Serialize ride estimates as sorted set members
    for count in [5, 20] {
        let estimates: Vec<RideEstimate> = (0..count)
            .map(|i| RideEstimate {
                ride_id: format!("ride-{i}"),
                distance_km: 5.0 + (i as f64 * 2.0),
                duration_min: 10.0 + (i as f64 * 5.0),
                fare: 100.0 + (i as f64 * 50.0),
                surge_multiplier: 1.0 + (i as f64 * 0.1),
                vehicle_type: "SEDAN".to_string(),
            })
            .collect();

        let jsons: Vec<String> = estimates
            .iter()
            .map(|e| serde_json::to_string(e).unwrap())
            .collect();

        group.bench_with_input(
            BenchmarkId::new("serialize_batch", count),
            &estimates,
            |b, items| {
                b.iter(|| {
                    items
                        .iter()
                        .map(|e| serde_json::to_string(black_box(e)))
                        .collect::<Result<Vec<_>, _>>()
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("deserialize_batch", count),
            &jsons,
            |b, items| {
                b.iter(|| {
                    items
                        .iter()
                        .map(|s| serde_json::from_str::<RideEstimate>(black_box(s)))
                        .collect::<Result<Vec<_>, _>>()
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_set_key_pattern,
    bench_get_key_pattern,
    bench_hash_operations,
    bench_geo_operations,
    bench_stream_operations,
    bench_sorted_set_operations
);
criterion_main!(benches);
