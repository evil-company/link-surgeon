//!
//! For either dev debug or log handling

const WATCHDOG_PING_PERIOD: Duration = Duration::from_secs(10);
const WATCHDOG_PING_TIMEOUT: Duration = Duration::from_secs(3);

use crate::app;
use crate::IDEF;
use std::time::Duration;

pub fn get_thread_id() -> String {
    format!("{:?}", std::thread::current().id())
}

/// Pings the main thread, logs if it doesn't answer within set time
///
/// I see no issues right now, but i suspect my code may be flaky,
/// and i imagine this might be nice to have in user logs.
/// So probably still best kept in production.
pub fn start_watchdog() {
    std::thread::spawn(move || loop {
        std::thread::sleep(WATCHDOG_PING_PERIOD);
        let (tx, rx) = std::sync::mpsc::channel();
        if app()
            .run_on_main_thread(move || {
                let _ = tx.send(());
            })
            .is_err()
        {
            log::warn!("{IDEF}: watchdog: run_on_main_thread failed");
            continue;
        }
        if rx.recv_timeout(WATCHDOG_PING_TIMEOUT).is_err() {
            log::error!(
                "{IDEF}: watchdog: main thread unresponsive for {WATCHDOG_PING_TIMEOUT:?}+"
            );
        }
    });
}
