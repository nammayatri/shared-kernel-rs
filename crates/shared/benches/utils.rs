use criterion::{black_box, criterion_group, criterion_main, Criterion};
use once_cell::sync::Lazy;
use regex::Regex;

// Matches the UUID regex used in middleware/utils.rs for path normalization.
static UUID_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}")
        .unwrap()
});

/// Benchmarks UUID regex replacement - the core operation in middleware get_path().
/// This runs on every incoming HTTP request to normalize paths for metrics.
fn bench_uuid_regex_replacement(c: &mut Criterion) {
    let mut group = c.benchmark_group("uuid_regex");

    // Path with single UUID (most common case)
    let path_single = "/api/v1/driver/550e8400-e29b-41d4-a716-446655440000/location";
    group.bench_function("single_uuid", |b| {
        b.iter(|| {
            UUID_REGEX
                .replace_all(black_box(path_single), ":id")
                .into_owned()
        })
    });

    // Path with multiple UUIDs
    let path_multi = "/api/v1/ride/550e8400-e29b-41d4-a716-446655440000/driver/660e8400-e29b-41d4-a716-446655440001";
    group.bench_function("multiple_uuids", |b| {
        b.iter(|| {
            UUID_REGEX
                .replace_all(black_box(path_multi), ":id")
                .into_owned()
        })
    });

    // Path with no UUID (should short-circuit quickly)
    let path_none = "/api/v1/health/check";
    group.bench_function("no_uuid", |b| {
        b.iter(|| {
            UUID_REGEX
                .replace_all(black_box(path_none), ":id")
                .into_owned()
        })
    });

    // Long path with multiple UUIDs
    let path_long = "/api/v2/org/550e8400-e29b-41d4-a716-446655440000/fleet/660e8400-e29b-41d4-a716-446655440001/driver/770e8400-e29b-41d4-a716-446655440002/trips";
    group.bench_function("long_path_three_uuids", |b| {
        b.iter(|| {
            UUID_REGEX
                .replace_all(black_box(path_long), ":id")
                .into_owned()
        })
    });

    group.finish();
}

/// Benchmarks URL decoding - used in get_path() on every incoming request.
fn bench_url_decoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("url_decoding");

    // Simple path (no encoding needed - common case)
    let simple = "/api/v1/driver/location";
    group.bench_function("no_encoding", |b| {
        b.iter(|| urlencoding::decode(black_box(simple)).map(|s| s.into_owned()))
    });

    // Encoded spaces
    let encoded = "/api/v1/search/New%20York%20City/rides";
    group.bench_function("encoded_spaces", |b| {
        b.iter(|| urlencoding::decode(black_box(encoded)).map(|s| s.into_owned()))
    });

    // Unicode encoded path (non-Latin script)
    let unicode = "/api/v1/route/%E0%B4%95%E0%B5%8B%E0%B4%9A%E0%B4%BF/to/%E0%B4%A4%E0%B4%BF%E0%B4%B0%E0%B5%81";
    group.bench_function("unicode_encoded", |b| {
        b.iter(|| urlencoding::decode(black_box(unicode)).map(|s| s.into_owned()))
    });

    group.finish();
}

/// Benchmarks the path parameter replacement pattern used in get_path().
/// match_info().iter() replaces each captured path segment.
fn bench_path_param_replacement(c: &mut Criterion) {
    let mut group = c.benchmark_group("path_param_replace");

    // Single parameter replacement
    group.bench_function("single_param", |b| {
        b.iter(|| {
            let mut path = "/api/v1/driver/driver-john-doe-123/location".to_string();
            path = path.replace(black_box("driver-john-doe-123"), black_box(":driverId"));
            path
        })
    });

    // Multiple parameter replacements (common in nested routes)
    group.bench_function("three_params", |b| {
        b.iter(|| {
            let mut path =
                "/api/v1/org/org-acme-456/driver/driver-john-123/ride/ride-789/status".to_string();
            path = path.replace(black_box("org-acme-456"), black_box(":orgId"));
            path = path.replace(black_box("driver-john-123"), black_box(":driverId"));
            path = path.replace(black_box("ride-789"), black_box(":rideId"));
            path
        })
    });

    group.finish();
}

/// Benchmarks the full get_path pipeline: URL decode -> param replace -> UUID regex.
fn bench_full_path_normalization(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_path_normalization");

    // Typical request path through the full pipeline
    let raw_path = "/api/v1/driver/550e8400-e29b-41d4-a716-446655440000/ride/New%20York";
    group.bench_function("typical_request", |b| {
        b.iter(|| {
            // Step 1: URL decode
            let mut path = urlencoding::decode(black_box(raw_path))
                .map(|s| s.into_owned())
                .unwrap_or_else(|_| raw_path.to_string());
            // Step 2: UUID regex replacement
            path = UUID_REGEX.replace_all(&path, ":id").into_owned();
            path
        })
    });

    // Clean path (no encoding, no UUIDs - best case)
    let clean_path = "/api/v1/health/check";
    group.bench_function("clean_path", |b| {
        b.iter(|| {
            let mut path = urlencoding::decode(black_box(clean_path))
                .map(|s| s.into_owned())
                .unwrap_or_else(|_| clean_path.to_string());
            path = UUID_REGEX.replace_all(&path, ":id").into_owned();
            path
        })
    });

    group.finish();
}

/// Benchmarks error message formatting patterns used in RedisError and CallAPIError.
fn bench_error_formatting(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_formatting");

    let err_detail = "Connection refused: redis.example.com:6379";
    group.bench_function("redis_error_format", |b| {
        b.iter(|| format!("Redis Error : {}", black_box(err_detail)))
    });

    // Error body JSON construction (matches ErrorBody serialization pattern)
    group.bench_function("error_body_json", |b| {
        b.iter(|| {
            serde_json::to_string(&serde_json::json!({
                "errorMessage": black_box("Connection refused"),
                "errorCode": black_box("REDIS_CONNECTION_FAILED")
            }))
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_uuid_regex_replacement,
    bench_url_decoding,
    bench_path_param_replacement,
    bench_full_path_normalization,
    bench_error_formatting
);
criterion_main!(benches);
