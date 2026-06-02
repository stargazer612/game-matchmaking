use crate::player::Player;
use crate::match_result::GameMatch;
use crate::metrics::Metrics;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

pub fn run_worker(
    worker_id: u64,
    pool: Arc<Mutex<Vec<Player>>>,
    metrics: Arc<Metrics>,
    stop_flag: Arc<AtomicBool>,
    match_tx: Sender<GameMatch>,
    match_id_counter: Arc<AtomicU64>,
) {
    println!("Worker {:02} started", worker_id);

    let mut idle_cycles = 0u64;

    while !stop_flag.load(Ordering::Relaxed) {
        match try_form_match(&pool, &metrics, &match_id_counter) {
            Some(game_match) => {
                idle_cycles = 0; 

                if match_tx.send(game_match).is_err() {
                    break; 
                }
            }
            None => {
                idle_cycles += 1;
                let sleep_ms = match idle_cycles {
                    1..=5 => 1,       
                    6..=20 => 5,
                    _ => 15,
                };
                thread::sleep(Duration::from_millis(sleep_ms));
            }
        }
    }

    println!("Worker {:02} stopped", worker_id);
}

fn try_form_match(
    pool: &Arc<Mutex<Vec<Player>>>,
    metrics: &Arc<Metrics>,
    match_id_counter: &Arc<AtomicU64>,
) -> Option<GameMatch> {
    let selected: Vec<Player> = {
        let mut guard = pool.lock().unwrap();

        if guard.len() < 10 {
            return None; 
        }

        guard.sort_by(|a, b| {
            a.skill_rating
                .partial_cmp(&b.skill_rating)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let indices = find_group_indices(&guard)?;

        let index_set: HashMap<usize> = indices.into_iter().collect();

        let mut selected = Vec::with_capacity(10);
        let mut remaining = Vec::with_capacity(guard.len().saturating_sub(10));

        for (i, player) in guard.drain(..).enumerate() {
            if index_set.contains_key(&i) {
                selected.push(player);
            } else {
                remaining.push(player);
            }
        }

        *guard = remaining;

        selected
    };

    let wait_times: Vec<u64> = selected.iter().map(|p| p.wait_ms()).collect();
    metrics.record_match_created(&wait_times);

    let (team_a, team_b) = balance_teams(&selected);

    let match_id = match_id_counter.fetch_add(1, Ordering::SeqCst);

    Some(GameMatch::new(match_id, team_a, team_b))
}

fn find_group_indices(sorted_players: &[Player]) -> Option<Vec<usize>> {
    let n = sorted_players.len();
    if n < 10 {
        return None;
    }

    for i in 0..=(n - 10) {
        let window = &sorted_players[i..(i + 10)];

        let min_acceptable_range = window.iter()
            .map(|p| p.acceptable_skill_range())
            .fold(f64::NEG_INFINITY, f64::max);

        let skill_spread = window[9].skill_rating - window[0].skill_rating;

        if skill_spread <= min_acceptable_range {
            return Some((i..(i + 10)).collect());
        }
    }

    None
}

fn balance_teams(players: Vec<Player>) -> (Vec<Player>, Vec<Player>) {
    debug_assert_eq!(players.len(), 10, "Must have exactly 10 players to balance teams");

    let mut best_diff = f64::INFINITY;
    let mut best_mask: u16 = 0b00000_11111; // first default: players 0-4 in team A

    for mask in 0u16..(1u16 << 10) {
        if mask.count_ones() != 5 {
            continue; 
        }

        let (sum_a, sum_b) = players.iter().enumerate().fold(
            (0.0f64, 0.0f64), 
            |(sa, sb), (i, player)| {
                if (mask & (1 << i)) != 0 {
                    (sa + player.skill_rating, sb) //Player i goes to team A
                } else {
                    (sa, sb + player.skill_rating)  //Player i goes to team B
                }
            },
        );

        let diff = (sum_a - sum_b).abs();
        if diff < best_diff {
            best_diff = diff;
            best_mask = mask;

            if diff < 1.0 {
                break; 
            }
        }
    }

    let (team_a_indexed, team_b_indexed): (Vec<_>, Vec<_>) = players
        .into_iter()
        .enumerate()
        .partition(|(i, _)| (best_mask & (1 << i)) != 0);

    let team_a: Vec<Player> = team_a_indexed.into_iter().map(|(_, p)| p).collect();
    let team_b: Vec<Player> = team_b_indexed.into_iter().map(|(_, p)| p).collect();

    (team_a, team_b)
}


#[cfg(test)]
mod tests {
    use super::*;
    
    fn player(id: u64, skill: f64) -> Player {
        Player::new(id, skill)
    }

    fn make_sorted_pool(rating: Vec<f64>) -> Vec<Player> {
        let mut players: Vec<Player> = rating.into_iter().enumerate()
            .map(|(i, &r)| player(i as u64, r))
            .collect();

        players.sort_by(|a, b| a.skill_rating.partial_cmp(&b.skill_rating).unwrap());

        players
    }

    #[test]
    fn test_finds_group_in_tight_clusters() {
        let pool = make_sorted_pool(vec![
            1460.0, 1465.0, 1470.0, 1475.0, 1480.0,
            1485.0, 1490.0, 1495.0, 1498.0, 1500.0,
        ]);

        let result = find_group_indices(&pool);
        assert!(result.is_some(), "Should find a group in a tight cluster");
    }

    #[test]
    fn no_group_if_spread_out_pool() {
        let pool = make_sorted_pool(vec![
            100.0, 300.0, 500.0, 700.0, 900.0,
            1100.0, 1300.0, 1500.0, 1700.0, 1900.0,
        ]);

        let result = find_group_indices(&pool);
        assert!(result.is_none(), "Should not find a group in a spread-out pool");
    }

    #[test]
    fn balance_teams_produces_equal_teams() {
        let players: Vec<Player> = (0..10).map(|i| {
            let skill = if i < 5 { 1000.0 } else { 2000.0 };
            player(i, skill)
        }).collect();

        let (team_a, team_b) = balance_teams(players);

        let sum_a: f64 = team_a.iter().map(|p| p.skill_rating).sum();
        let sum_b: f64 = team_b.iter().map(|p| p.skill_rating).sum();

        assert!(
            (sum_a - sum_b).abs() < 1001.0, 
            "Teams should be balanced as possible, {} vs {}", sum_a, sum_b
        );
    }
}