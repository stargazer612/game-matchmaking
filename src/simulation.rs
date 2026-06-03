use crate::metrics::Metrics;
use crate::player::Player;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use rand::Rng;

pub fn run_simulation(
    pool: Arc<Mutex<Vec<Player>>>,
    metrics: Arc<Metrics>,
    total_players: u64,
    concurrency: usize,
    players_per_second: f64,
) {
    println!(
        "Starting simulation with {} players, concurrency {}, rate {:.2} p/s",
        total_players, concurrency, players_per_second
    );

    let id_counter = Arc::new(AtomicU64::new(1));

    let player_per_thread = total_players / concurrency as u64;

    let interval = Duration::from_secs_f64(concurrency as f64 / players_per_second);

    let start = Instant::now();

    let handles: Vec<_> = (0..concurrency)
        .map(|_thread_idx| {
            let pool_clone = Arc::clone(&pool);
            let metrics_clone = Arc::clone(&metrics);
            let id_counter_clone = Arc::clone(&id_counter);

            thread::spawn(move || {
                let mut rng = rand::thread_rng();

                for _ in 0..player_per_thread {
                    let id = id_counter_clone.fetch_add(1, Ordering::Relaxed);

                    let skill = generate_skill_rating(&mut rng);

                    let player = Player::new(id, skill);

                    {
                        pool_clone.lock().unwrap().push(player);
                    }

                    metrics_clone.record_player_queued();

                    thread::sleep(interval);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Simulation thread panicked");
    }

    let elapsed = start.elapsed();
    let total_injected = id_counter.load(Ordering::Relaxed) - 1;

    println!(
        "\n Simulation completed: injected {} players in {:.2?} (actual rate {:.2} p/s)",
        total_injected,
        elapsed.as_secs_f64(),
        total_injected as f64 / elapsed.as_secs_f64()
    );
}

fn generate_skill_rating(rng: &mut impl Rng) -> f64 {
    let u1: f64 = rng.r#gen::<f64>().max(1e-10);
    let u2: f64 = rng.r#gen::<f64>();

    let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();

    let mean = 1500.0;
    let std_dev = 400.0;

    (mean + z * std_dev).clamp(100.0, 3000.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::thread_rng;

    #[test]
    fn test_skill_distribution_is_centered_around_1500() {
        let mut rng = thread_rng();
        let samples: Vec<f64> = (0..10_000)
            .map(|_| generate_skill_rating(&mut rng))
            .collect();

        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        let min = samples.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = samples.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        assert!(
            (mean - 1500.0).abs() < 50.0,
            "Mean {:.1} is too far from 1500",
            mean
        );

        assert!(min >= 100.0, "Min {} below 100", min);
        assert!(max <= 3000.0, "Max {} above 3000", max);
    }
}
