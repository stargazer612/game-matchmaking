mod match_result;
mod matchmaker;
mod metrics;
mod player;
mod simulation;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use match_result::GameMatch;
use matchmaker::run_worker;
use metrics::Metrics;
use player::Player;
use simulation::run_simulation;

const NUM_WORKERS: usize = 4;
const TOTAL_PLAYERS: u64 = 3_000;
const SIM_THREADS: usize = 15;
const PLAYERS_PER_SECOND: f64 = 300.0;

fn main() {
    print_banner();
    let pool: Arc<Mutex<Vec<Player>>> = Arc::new(Mutex::new(Vec::new()));
    let metrics = Metrics::new();
    let stop_flag = Arc::new(AtomicBool::new(false));
    let match_counter = Arc::new(AtomicU64::new(1));

    let (match_tx, match_rx) = mpsc::channel::<GameMatch>();

    println!("\n  Starting {} matchmaker workers...", NUM_WORKERS);

    let worker_handles: Vec<_> = (0..NUM_WORKERS as u64)
        .map(|id| {
            let (p, m, s, tx, c) = (
                Arc::clone(&pool),
                Arc::clone(&metrics),
                Arc::clone(&stop_flag),
                match_tx.clone(),
                Arc::clone(&match_counter),
            );
            thread::spawn(move || run_worker(id, p, m, s, tx, c))
        })
        .collect();

    // ── Match Result Handler ──────────────────────────────────────────────────
    thread::spawn(move || {
        for game_match in match_rx {
            print_match_result(&game_match);
        }
    });

    {
        let m = Arc::clone(&metrics);
        let s = Arc::clone(&stop_flag);
        thread::spawn(move || {
            while !s.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_secs(3));
                m.report();
            }
        });
    }

    println!(
        "  Injecting {} players at {}/sec...",
        TOTAL_PLAYERS, PLAYERS_PER_SECOND
    );
    run_simulation(
        Arc::clone(&pool),
        Arc::clone(&metrics),
        TOTAL_PLAYERS,
        SIM_THREADS,
        PLAYERS_PER_SECOND,
    );

    println!("\n  Waiting for pool to drain...");
    let drain_start = std::time::Instant::now();
    loop {
        let sz = pool.lock().unwrap().len();
        if sz < 10 {
            println!("  Pool drained ({} left).", sz);
            break;
        }
        if drain_start.elapsed().as_secs() > 30 {
            println!("  Drain timeout — {} players still waiting.", sz);
            break;
        }
        thread::sleep(Duration::from_millis(200));
    }

    stop_flag.store(true, Ordering::SeqCst);
    drop(match_tx); // drop main thread's sender → channel closes when workers exit
    for h in worker_handles {
        let _ = h.join();
    }

    println!("\n  ════════════════ FINAL REPORT ════════════════");
    metrics.report();

    let leftover = pool.lock().unwrap().len();
    if leftover > 0 {
        println!(
            "  ℹ  {} players remain (< 10, can't form a full game)",
            leftover
        );
    } else {
        println!("All players matched!");
    }
    println!();
}

fn print_match_result(gm: &GameMatch) {
    let avg = |t: &[Player]| t.iter().map(|p| p.skill_rating).sum::<f64>() / t.len() as f64;
    let filled = (gm.quality_score * 10.0).round() as usize;
    let bar = format!(
        "[{}{}]",
        "█".repeat(filled.min(10)),
        "░".repeat(10 - filled.min(10))
    );
    println!(
        "Match #{:04} │ A:{:>5.0} vs B:{:>5.0} │ Q:{:.3} {} │ Wait:{:>6.0}ms",
        gm.id,
        avg(&gm.team_a),
        avg(&gm.team_b),
        gm.quality_score,
        bar,
        gm.avg_wait_ms
    );
}

fn print_banner() {
    println!();
    println!("  ╔═══════════════════════════════════════════════════╗");
    println!("  ║         5v5 Competitive Matchmaking Engine        ║");
    println!("  ║   Rust  │  Multi-threaded  │  Lock-free Metrics   ║");
    println!("  ╚═══════════════════════════════════════════════════╝");
}
