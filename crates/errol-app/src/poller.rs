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
    let spawned = thread::Builder::new()
        .name("poller".into())
        .spawn(move || poll(&observe));
    // without a poller the screen keeps the first snapshot: still worth showing
    if let Err(error) = spawned {
        eprintln!("errol: no poller thread: {error}");
    }
}

/// The poller's loop: follows the stream when it can, asks every second when not.
fn poll(observe: &Arc<ObserveFeed>) {
    // nothing heard yet: the stream is tried at once
    let heard = Arc::new(Mutex::new(
        Instant::now()
            .checked_sub(STALE)
            .unwrap_or_else(Instant::now),
    ));
    let following = Arc::new(AtomicBool::new(false));
    let mut tried: Option<Instant> = None;
    loop {
        let heard_ago = heard
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .elapsed();
        if should_try_stream(heard_ago, tried.map(|at| at.elapsed())) {
            tried = Some(Instant::now());
            follow(observe.clone(), heard.clone(), following.clone());
        }
        if should_ask(heard_ago, following.load(Ordering::Relaxed)) {
            observe.refresh();
        }
        thread::sleep(INTERVAL);
    }
}

/// The stream is tried when it has been quiet too long, and not tried lately.
fn should_try_stream(heard_ago: Duration, tried_ago: Option<Duration>) -> bool {
    heard_ago >= STALE && tried_ago.is_none_or(|ago| ago >= RETRY)
}

/// The poller asks by itself while the stream is quiet or not followed.
fn should_ask(heard_ago: Duration, following: bool) -> bool {
    heard_ago >= STALE || !following
}

/// Follows the stream on its own thread until it ends; a stream that hangs
/// (a cable pulled) stays quiet, and the poller takes over after `STALE`.
fn follow(observe: Arc<ObserveFeed>, heard: Arc<Mutex<Instant>>, following: Arc<AtomicBool>) {
    let spawned = thread::Builder::new()
        .name("stream".into())
        .spawn(move || stream(&observe, &heard, &following));
    if let Err(error) = spawned {
        eprintln!("errol: no stream thread: {error}");
    }
}

/// The stream's thread: takes what it says until it ends, then says why
/// if it was followed.
fn stream(observe: &ObserveFeed, heard: &Mutex<Instant>, following: &AtomicBool) {
    let ended = observe.follow(&mut || mark(following, heard));
    if following.swap(false, Ordering::Relaxed)
        && let Err(error) = ended
    {
        eprintln!("errol: stream: {error}");
    }
}

/// The stream said something: it is followed, and heard now.
fn mark(following: &AtomicBool, heard: &Mutex<Instant>) {
    *heard.lock().unwrap_or_else(PoisonError::into_inner) = Instant::now();
    if !following.swap(true, Ordering::Relaxed) {
        eprintln!("errol: following the feed's stream");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stream_is_tried_once_quiet_and_not_tried_lately() {
        assert!(should_try_stream(STALE, None));
        assert!(should_try_stream(STALE, Some(RETRY)));
        assert!(!should_try_stream(STALE, Some(Duration::from_secs(1))));
        assert!(!should_try_stream(Duration::from_secs(3), None));
    }

    #[test]
    fn the_poller_asks_unless_a_live_stream_is_followed() {
        assert!(should_ask(Duration::from_secs(1), false));
        assert!(should_ask(STALE, true));
        assert!(!should_ask(Duration::from_secs(1), true));
    }

    #[test]
    fn a_word_from_the_stream_marks_it_followed_and_heard() {
        let following = AtomicBool::new(false);
        let long_ago = Instant::now()
            .checked_sub(STALE)
            .unwrap_or_else(Instant::now);
        let heard = Mutex::new(long_ago);
        mark(&following, &heard);
        mark(&following, &heard);
        assert!(following.load(Ordering::Relaxed));
        assert!(heard.lock().unwrap().elapsed() < STALE);
    }
}
