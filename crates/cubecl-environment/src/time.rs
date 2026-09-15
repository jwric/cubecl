#[cfg(not(target_os = "none"))]
pub use web_time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[cfg(target_os = "none")]
pub use embassy_time::{Duration, Instant};
