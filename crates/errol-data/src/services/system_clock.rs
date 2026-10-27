use std::time::{SystemTime, UNIX_EPOCH};

use errol_domain::entities::Timestamp;
use errol_domain::ports::Clock;

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        Timestamp(elapsed.as_millis() as i64)
    }
}
