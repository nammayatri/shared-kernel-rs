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
