//! Reads the feed on its own threads, so a slow answer never stalls a frame.
//! The feed pushes every change on its event stream: one thread follows it,
//! and a change shows the instant the feed writes it. While there is no
//! stream (the demo, a feed without one, a connection that broke), the
//! poller asks every second, and tries the stream again now and then.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use errol_domain::usecases::ObserveFeed;

/// The feed service writes every 3 s; reading every second adds at most a
/// second to that before an event takes the screen.
const INTERVAL: Duration = Duration::from_secs(1);
/// The stream says something at least every 15 s (a comment when the feed is
/// quiet): longer than this without a word, and the poller asks by itself.
const STALE: Duration = Duration::from_secs(45);
/// How long after a stream ended (or went quiet) the next one is tried.
const RETRY: Duration = Duration::from_secs(15);

pub fn start(observe: Arc<ObserveFeed>) {
    observe.refresh();
    let spawned = thread::Builder::new().name("poller".into()).spawn(move || {
        // nothing heard yet: the stream is tried at once
        let heard = Arc::new(Mutex::new(
            Instant::now()
                .checked_sub(STALE)
                .unwrap_or_else(Instant::now),
        ));
        let following = Arc::new(AtomicBool::new(false));
        let mut tried: Option<Instant> = None;
        loop {
            let last = *heard.lock().unwrap_or_else(PoisonError::into_inner);
            let quiet = last.elapsed() >= STALE;
            if quiet && tried.is_none_or(|at| at.elapsed() >= RETRY) {
                tried = Some(Instant::now());
                follow(observe.clone(), heard.clone(), following.clone());
            }
            if quiet || !following.load(Ordering::Relaxed) {
                observe.refresh();
            }
            thread::sleep(INTERVAL);
        }
    });
    // without a poller the screen keeps the first snapshot: still worth showing
    if let Err(error) = spawned {
        eprintln!("errol: no poller thread: {error}");
    }
}

/// Follows the stream on its own thread until it ends; a stream that hangs
/// (a cable pulled) stays quiet, and the poller takes over after `STALE`.
fn follow(observe: Arc<ObserveFeed>, heard: Arc<Mutex<Instant>>, following: Arc<AtomicBool>) {
    let spawned = thread::Builder::new().name("stream".into()).spawn(move || {
        let mark = |following: &AtomicBool, heard: &Mutex<Instant>| {
            *heard.lock().unwrap_or_else(PoisonError::into_inner) = Instant::now();
            if !following.swap(true, Ordering::Relaxed) {
                eprintln!("errol: following the feed's stream");
            }
        };
        let ended = observe.follow(&mut || mark(&following, &heard));
        if following.swap(false, Ordering::Relaxed)
            && let Err(error) = ended
        {
            eprintln!("errol: stream: {error}");
        }
    });
    if let Err(error) = spawned {
        eprintln!("errol: no stream thread: {error}");
    }
}
