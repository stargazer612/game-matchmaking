# 5v5 Competitive Matchmaking Engine

A high-performance, thread-safe matchmaking engine built in Rust.
Groups players into balanced 5v5 matches based on skill rating (MMR),
with time-based constraint relaxation to prevent players from waiting forever.

## Demo
Match #0001 │ A: 1256 vs B: 1256 │ Q:0.980 [██████████] │ Wait:  173ms  
Match #0002 │ A: 1315 vs B: 1315 │ Q:0.980 [██████████] │ Wait:  163ms  
Match #0034 │ A: 2048 vs B: 2048 │ Q:0.985 [██████████] │ Wait:  675ms  
┌─ METRICS ──────────────────────────────────────────┐  
│  Queued:   3000  │  Matched:  2960 (98%)  │  Pool: 40  │  
│  Matches:   296  │  Avg Wait:     480.0 ms            │    
└────────────────────────────────────────────────────┘

## Features

- **Thread-safe player pool** using `Arc<Mutex<Vec<Player>>>`
- **Sliding window algorithm** to find 10 skill-compatible players
- **Time-based constraint relaxation** — prevents players from waiting forever
- **Optimal team balancing** — brute-forces all C(10,5) = 252 splits to find fairest 5v5
- **Lock-free metrics** using atomic operations (`AtomicU64`) — zero impact on throughput
- **Concurrent simulation** — injects thousands of players via a normal (bell curve) skill distribution

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs) (stable)

### Run

```bash
git clone https://github.com/stargazer612/game-matchmaking.git
cd matchmaker
cargo run --release
```

### Test

```bash
cargo test
```

## Project Structure
matchmaker/  
├── Cargo.toml  
└── src/  
    ├── main.rs           # Entry point — wires all threads together  
    ├── player.rs         # Player struct + time-based skill relaxation  
    ├── match_result.rs   # GameMatch struct + quality score formula  
    ├── metrics.rs        # Lock-free atomic health metrics  
    ├── matchmaker.rs     # Core algorithm: sliding window + team balancer  
    └── simulation.rs     # Concurrent load injection (Box-Muller distribution)  

## Architecture
Simulation Threads (×15)  
│  
▼  
Arc<Mutex<Vec<Player>>>  ←──────────────────────────────┐  
(Player Pool)                                      │  
│                                               │  
├──► Worker 0  ─┐                               │  
├──► Worker 1  ─┤                               │  
├──► Worker 2  ─┼──► mpsc channel ──► Result Handler (prints matches)  
└──► Worker 3  ─┘  

Metrics Reporter (every 3s, lock-free)
Each worker independently:
1. Locks the pool
2. Sorts players by skill rating
3. Scans with a sliding window of 10
4. Atomically removes a compatible group
5. Releases the lock
6. Balances teams outside the lock

## How the Algorithm Works

### Step 1 — Sliding Window Search

Players are sorted by skill rating. A window of 10 slides across the sorted list.
A group is valid when the skill spread fits within every player's acceptable range. 
Sorted pool: [820, 900, 1440, 1460, 1470, 1480, 1490, 1500, 1510, 1520, 2100]  
Window [0..10]: spread = 1520 - 820 = 700  → too wide ✗  
Window [1..11]: spread = 2100 - 900 = 1200 → too wide ✗  
Window [2..12]: spread = 1520 - 1440 = 80  → within range ✓  → MATCH  

### Step 2 — Time-Based Constraint Relaxation

The longer a player waits, the more skill difference they accept:

| Wait Time | Acceptable Range | Purpose |
|-----------|-----------------|---------|
| 0 – 9s    | ± 50 MMR        | Tight — high quality match |
| 10 – 29s  | ± 150 MMR       | Normal |
| 30 – 59s  | ± 300 MMR       | Relaxed — player is impatient |
| 60s+      | ± 1500 MMR      | Match anyone — prevent starvation |

The **most restrictive player** in a window sets the limit for the whole group.

### Step 3 — Team Balance

All C(10,5) = 252 possible 5v5 splits are evaluated using bitmasks.
The split with the smallest `|avg(Team A) - avg(Team B)|` is chosen.

```rust
for mask in 0u16..(1u16 << 10) {       // 1024 iterations
    if mask.count_ones() != 5 { continue; } // only 252 valid splits
    let diff = (sum_a - sum_b).abs();
    if diff < best_diff { best_mask = mask; }
}
```

This runs in ~200 nanoseconds — brute force is perfectly viable at this scale.

### Match Quality Score
quality = balance_score × 0.70 + spread_score × 0.30  
balance_score = (1 - |avg_A - avg_B| / 200).max(0)  
spread_score  = (1 - avg_std_dev / 300).max(0)  

Most matches in the simulation score **0.97 – 0.99**.

## Complexity Analysis

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| Player insertion | O(1) | Append to Vec |
| Pool sort | O(n log n) | Per worker scan |
| Sliding window | O(n) | Per worker scan |
| Team balancing | O(1) | Always 252 × 10 ops |
| Metrics update | O(1) | Single atomic instruction |

**Space:** O(n) for pool + O(m) for unprocessed match results.

## Simulation Results

| Metric | Value |
|--------|-------|
| Players injected | 3,000 |
| Match rate | 99% |
| Avg wait time (steady state) | ~480ms |
| Quality score | 0.97 – 0.99 |
| Throughput | ~30 matches/sec |

## Scaling Considerations

| Challenge | Solution |
|-----------|---------|
| Mutex contention with many workers | Shard pool by MMR tier |
| Single machine memory limit | Distribute pool across Redis sorted sets |
| Extreme MMR starvation | Virtual population padding for sparse tiers |
| CPU waste on empty pool | Replace sleep with `Condvar` wake signal |
