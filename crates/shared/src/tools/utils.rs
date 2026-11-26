use crate::termination;
use crate::tools::prometheus::TERMINATION;
use std::time::Instant;
use tracing::error;

pub fn set_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        termination!("panic", Instant::now());
        let payload = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.to_string()
        } else {
            "Unknown".to_string()
        };
        error!("Panic Occured : {} - {:?}", payload, panic_info);
    }));
}

const SENSITIVE_HEADER_MARKERS: [&str; 8] = [
    "token",
    "authorization",
    "api-key",
    "apikey",
    "secret",
    "password",
    "cookie",
    "session",
];

pub fn is_sensitive_header(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    SENSITIVE_HEADER_MARKERS
        .iter()
        .any(|marker| name.contains(marker))
}

pub fn redact_headers<I, K, V>(headers: I) -> String
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<str>,
    V: AsRef<[u8]>,
{
    let mut out = String::from("{");
    for (idx, (name, value)) in headers.into_iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        let name = name.as_ref();
        let value = if is_sensitive_header(name) {
            "[REDACTED]".to_string()
        } else {
            String::from_utf8_lossy(value.as_ref()).into_owned()
        };
        out.push_str(&format!("{name:?}: {value:?}"));
    }
    out.push('}');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_sensitive_headers_and_keeps_others() {
        let headers = vec![
            ("token", "fake-driver-token"),
            ("api-key", "fake-internal-key"),
            ("X-Goog-Api-Key", "fake-google-key"),
            ("Authorization", "Bearer fake-jwt"),
            ("user-agent", "okhttp/4.12.0"),
        ];
        let out = redact_headers(headers);
        assert!(!out.contains("fake-"));
        assert!(out.contains("\"user-agent\": \"okhttp/4.12.0\""));
        assert!(out.contains("\"token\": \"[REDACTED]\""));
    }

    #[test]
    fn redacts_reqwest_header_map() {
        let mut map = reqwest::header::HeaderMap::new();
        map.insert("api-key", "fake-value".parse().unwrap());
        map.insert("content-type", "application/json".parse().unwrap());
        let out = redact_headers(map.iter());
        assert!(!out.contains("fake-value"));
        assert!(out.contains("application/json"));
    }
}
