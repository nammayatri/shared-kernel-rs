# Changelog

## [Unreleased]

### Performance

- **redis/error**: Return `&'static str` from `code()` instead of allocating a `String` on every error
- **redis/error**: Consolidate match arms in `message()` using `|` to eliminate repeated `format!` calls
- **redis/error**: Clone inner strings directly instead of going through `.to_string()` indirection
- **redis/error**: Add `#[inline]` hints on `error_message()`, `message()`, `code()`, `error_response()`, `status_code()`
- **redis/types**: Use struct literal for `PerformanceConfig` instead of default + mutation
- **redis/types**: Add `#[inline]` on `Deref` impl for `RedisClient`
- **callapi**: Return `&'static str` from `code()` instead of allocating a `String` on every error
- **callapi**: Add `#[inline]` hints on error and response methods
- **callapi**: Use `unwrap_or_else` with `to_owned()` instead of `unwrap_or` with `to_string()`
- **middleware/utils**: Compile UUID regex once via `once_cell::sync::Lazy` instead of per-request
- **middleware/utils**: Accept `&str` in `calculate_metrics` instead of taking ownership of `String` args
- **middleware/utils**: Add `#[inline]` on `get_method()` and `get_headers()`
- **middleware/incoming_request**: Extract headers/method/path into locals before passing by reference
- **prometheus**: Use `unwrap_or_else` with `to_owned()` in `incoming_api!` and `termination!` macros
- **logger**: Avoid unnecessary `.to_string()` on `concat!` macro result (already `&'static str`)
- **aws**: Replace `.to_owned()` on cloned response contents; use `to_owned()` over `to_string()` for `&str`
- **ErrorBody** (redis, callapi): Add `#[serde(skip_serializing_if = "String::is_empty")]` to skip empty fields during serialization

### Fixed

- **macros**: `measure_duration` proc macro now preserves function attributes (`#[allow(...)]`, `#[cfg(...)]`, etc.)
- **redis/commands**: Add explicit `RedisValue` type annotation on `pipeline.all()` and `xadd` calls to resolve type inference
- **redis/commands**: Use `.first()` instead of `.get(0)` (Clippy `first` lint)

### Changed

- **aws**: Use `aws_config::defaults(BehaviorVersion::latest())` instead of deprecated `aws_config::from_env()`
- **callapi**: Extract `ErrorHandler<E>` type alias for the boxed error handler function pointer
- **cloud_storage**: Add `#[allow(clippy::should_implement_trait)]` on deprecated `from_str` method

### Removed

- **redis/types**: Remove unused imports (`log::info`, `tracing::*`, `UnboundedReceiver`, `UnboundedSender`)

### Added

- **benchmarks**: Add criterion benchmarks for serialization patterns (`serialization.rs`)
- **benchmarks**: Add criterion benchmarks for middleware path normalization (`utils.rs`)
- **benchmarks**: Add criterion benchmarks for Redis operation patterns (`redis_patterns.rs`)
