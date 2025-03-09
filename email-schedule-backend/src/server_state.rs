use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, atomic::Ordering};
use std::num::NonZeroUsize;
use std::io::Read;

use tokio::time::{interval, Duration};
use tokio::sync::watch;

use indexmap::IndexMap;
use lru::LruCache;

const MAX_LRU_CAPACITY: NonZeroUsize = NonZeroUsize::new(1000).unwrap();
const STATE_TIMEOUT: u64 = 60;

type CacheRef = Arc<Mutex<LruCache<String, Arc<Vec<u8>>>>>;
type StateSet = Arc<Mutex<IndexMap<String, u64>>>;


pub struct ServerState {
    pub lru: CacheRef,
    pub states: StateSet,
    running: Arc<AtomicBool>,
    shutdown_tx: watch::Sender<()>,
    shutdown_rx: watch::Receiver<()>,
}
impl ServerState {
    pub fn new() -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(());
        ServerState {
            lru: Arc::new(Mutex::new(LruCache::new(MAX_LRU_CAPACITY))),
            states: Arc::new(Mutex::new(IndexMap::new())),
            running: Arc::new(AtomicBool::new(true)),
            shutdown_tx,
            shutdown_rx,
        }
    }

    pub fn check_state(&self, state: &str) -> bool {
        let mut states = self.states.lock().unwrap();
        if let Some(_) = states.get(state) {
            states.shift_remove(state);
            true
        }
        else{
            false
        }
    }

    pub fn get_from_cache(&self, path: &str) -> Result<Arc<Vec<u8>>, std::io::Error> {
        let mut guard = self.lru.lock().unwrap();
        if let Some(value) = guard.get(path) {
            Ok(value.clone())
        }
        else{
            drop(guard);
            let file = std::fs::File::open(path);
            match file {
                Ok(mut f) => {
                    let mut buffer = Vec::with_capacity(f.metadata()?.len() as usize);
                    f.read_to_end(&mut buffer)?;
                    let arc_buffer = Arc::new(buffer);
                    let mut guard = self.lru.lock().unwrap();
                    guard.put(path.to_string(), arc_buffer.clone());
                    Ok(arc_buffer)
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn now() -> u64 {
        let now = std::time::SystemTime::now();
        let duration = now.duration_since(std::time::UNIX_EPOCH).unwrap();
        duration.as_secs()
    }    
    pub fn add_state(&self, state: &str) {
        let mut states = self.states.lock().unwrap();
        states.insert(state.to_string(), Self::now());
    }

    #[inline]
    fn clean_states(&self) {
        let mut states = self.states.lock().unwrap();
        let now = Self::now();
        states.retain(|_, &mut v| now - v < STATE_TIMEOUT);
    }

    pub fn stop_maintenance(&self) {
        self.running.store(false, Ordering::SeqCst);
        let _ = self.shutdown_tx.send(());  // Signal shutdown
    }

    pub async fn maintenance_thread(state: Arc<ServerState>) {
        let mut interval = interval(Duration::from_secs(STATE_TIMEOUT));
        let mut shutdown_rx = state.shutdown_rx.clone();

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    state.clean_states();
                }
                _ = shutdown_rx.changed() => {
                    eprintln!("Stopping maintenance thread");
                    break;
                }
            }
        }
    }

}