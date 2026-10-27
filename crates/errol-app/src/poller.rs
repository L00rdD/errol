//! Reads the feed on its own thread, so a slow answer never stalls a frame.

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use errol_domain::usecases::ObserveFeed;

/// The feed service writes every 3 s; reading every 2 s keeps the lag under that.
const INTERVAL: Duration = Duration::from_secs(2);

pub fn start(observe: Arc<ObserveFeed>) {
    observe.refresh();
    let spawned = thread::Builder::new().name("poller".into()).spawn(move || {
        loop {
            thread::sleep(INTERVAL);
            observe.refresh();
        }
    });
    // without a poller the screen keeps the first snapshot: still worth showing
    if let Err(error) = spawned {
        eprintln!("errol: no poller thread: {error}");
    }
}
