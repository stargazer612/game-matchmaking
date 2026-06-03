use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

//Metrics struct to track matchmaking performance
pub struct Metrics {
    pub players_queued: AtomicU64, //Total number of players who have entered the matchmaking queue
    pub matches_created: AtomicU64, //Total number of matches created
    pub players_matched: AtomicU64, //Total number of players successfully matched into games
    pub total_wait_ms: AtomicU64,  //Cumulative wait time of all players (in milliseconds)
    pub current_pool_size: AtomicU64, //Current number of players waiting in the matchmaking queue
}

impl Metrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Metrics {
            players_queued: AtomicU64::new(0),
            matches_created: AtomicU64::new(0),
            players_matched: AtomicU64::new(0),
            total_wait_ms: AtomicU64::new(0),
            current_pool_size: AtomicU64::new(0),
        })
    }

    pub fn record_player_queued(&self) {
        self.players_queued.fetch_add(1, Ordering::Relaxed);
        self.current_pool_size.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_match_created(&self, wait_times_ms: &[u64]) {
        self.matches_created.fetch_add(1, Ordering::Relaxed);
        self.players_matched.fetch_add(10, Ordering::Relaxed);
        self.current_pool_size.fetch_sub(10, Ordering::Relaxed);

        let total_wait: u64 = wait_times_ms.iter().sum();
        self.total_wait_ms.fetch_add(total_wait, Ordering::Relaxed);
    }

    pub fn avg_wait_time(&self) -> f64 {
        let total_players = self.players_matched.load(Ordering::Relaxed);
        if total_players == 0 {
            0.0
        } else {
            self.total_wait_ms.load(Ordering::Relaxed) as f64 / total_players as f64
        }
    }

    // Throughput: percentage of queued players that got matched
    pub fn match_rate_pct(&self) -> f64 {
        let queued = self.players_queued.load(Ordering::Relaxed);
        if queued == 0 {
            0.0
        } else {
            (self.players_matched.load(Ordering::Relaxed) as f64 / queued as f64) * 100.0
        }
    }

    pub fn report(&self) {
        println!("--- Matchmaking Metrics ---");
        println!(
            "Players Queued: {}",
            self.players_queued.load(Ordering::Relaxed)
        );
        println!(
            "Matches Created: {}",
            self.matches_created.load(Ordering::Relaxed)
        );
        println!(
            "Players Matched: {}",
            self.players_matched.load(Ordering::Relaxed)
        );
        println!(
            "Average Wait Time: {:.2} seconds",
            self.avg_wait_time() / 1000.0
        );
        println!("Match Rate: {:.2}%", self.match_rate_pct());
        println!(
            "Current Pool Size: {}",
            self.current_pool_size.load(Ordering::Relaxed)
        );
    }
}
