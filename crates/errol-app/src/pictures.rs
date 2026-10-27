//! Posters and backdrops, fetched and decoded on a worker thread. The renderer
//! asks every frame (`take`); the first ask queues the picture, a later ask
//! gets it once it is there. A picture that failed is asked for again after
//! `RETRY`; one the renderer let go (`forget`) is fetched again when wanted.

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use errol_domain::entities::{Picture, PictureRef};
use errol_domain::usecases::LoadPicture;

/// A picture and the width it is wanted at.
pub type Key = (PictureRef, u32);

/// A failed picture (network down, Jellyfin restarting) is tried again after this.
const RETRY: Duration = Duration::from_mins(5);

/// Where a picture the renderer asked for is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    /// Queued or being fetched.
    Pending,
    /// Failed; asked for again from this moment.
    Failed { retry_at: Instant },
}

pub struct Pictures {
    requests: Sender<Key>,
    /// Pictures asked for and not handed over yet; handed-over ones leave it.
    status: Arc<Mutex<HashMap<Key, Status>>>,
    ready: Arc<Mutex<HashMap<Key, Picture>>>,
}

impl Pictures {
    pub fn start(load: LoadPicture) -> Self {
        let (requests, queue): (Sender<Key>, Receiver<Key>) = channel();
        let ready = Arc::new(Mutex::new(HashMap::new()));
        let status = Arc::new(Mutex::new(HashMap::new()));
        let (shelf, ledger) = (ready.clone(), status.clone());
        let spawned = thread::Builder::new()
            .name("pictures".into())
            .spawn(move || {
                for key in queue {
                    match load.execute(&key.0, key.1) {
                        Ok(picture) => {
                            shelf
                                .lock()
                                .unwrap_or_else(PoisonError::into_inner)
                                .insert(key, picture);
                        }
                        Err(error) => {
                            eprintln!("errol: {error} (next try in {} min)", RETRY.as_secs() / 60);
                            let retry_at = Instant::now() + RETRY;
                            ledger
                                .lock()
                                .unwrap_or_else(PoisonError::into_inner)
                                .insert(key, Status::Failed { retry_at });
                        }
                    }
                }
            });
        if let Err(error) = spawned {
            eprintln!("errol: no picture thread, pictures stay blank: {error}");
        }
        Self {
            requests,
            status,
            ready,
        }
    }

    /// The decoded picture, taken out of the shelf (the renderer keeps it on
    /// the GPU from then on); None while it loads or after it failed.
    pub fn take(&self, picture: &PictureRef, width: u32) -> Option<Picture> {
        let key = (picture.clone(), width);
        let mut status = self.status.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(decoded) = self
            .ready
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&key)
        {
            status.remove(&key);
            return Some(decoded);
        }
        let ask = match status.get(&key) {
            None => true,
            Some(Status::Failed { retry_at }) => Instant::now() >= *retry_at,
            Some(Status::Pending) => false,
        };
        if ask {
            status.insert(key.clone(), Status::Pending);
            // a closed channel only means no worker: the picture stays blank
            let _ = self.requests.send(key);
        }
        None
    }

    /// The renderer dropped this picture from the GPU: the next `take` fetches
    /// it again instead of waiting for something already handed over.
    pub fn forget(&self, picture: &PictureRef, width: u32) {
        let key = (picture.clone(), width);
        self.status
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&key);
        self.ready
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&key);
    }
}
