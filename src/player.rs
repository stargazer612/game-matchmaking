use std::time::Instant;

//Player struct
#[derive(Debug, Clone)]
pub struct Player {
    pub id: u64,                //Unique player ID
    pub skill_rating: f64,      //Player's skill rating for matchmaking; MMR (Matchmaking Rating): 100-3000, where higher is better
    pub joined_at: Instant,     //Time when the player joined the matchmaking queue
}

impl Player {
    //Create a new player joining the matchmaking queue
    pub fn new(id: u64, skill_rating: f64) -> Self {
        Player {
            id,
            skill_rating,
            joined_at: Instant::now(),
        }
    }

    //How many seconds the player has been waiting in the matchmaking queue?
    pub fn wait_time(&self) -> f64 {
        self.joined_at.elapsed().as_secs_f64()
    }

    //How many milliseconds the player has been waiting in the matchmaking queue?
    pub fn wait_ms(&self) -> u64 {
        self.joined_at.elapsed().as_millis() as u64
    }

    // TIME-BASED CONSTRAINT RELAXATION:
    // Challenge: "High-skill or low-skill players might wait
    // indefinitely if no perfect match exists."
    // Solution: The longer a player waits, the MORE skill difference they're
    // willing to accept. This prevents "starvation" (waiting forever).
    // Tiers:
    //   0–9s    → ±50  MMR   (tight — only near-perfect matches)
    //   10–29s  → ±150 MMR   (normal — acceptable difference)
    //   30–59s  → ±300 MMR   (relaxed — player is getting impatient)
    //   60s+    → ±1500 MMR  (desperate — match with ANYONE)
    
    pub fn acceptable_skill_range(&self) -> f64 {
        match self.wait_time() as u64 {
            0..=9 => 50.0,
            10..=29 => 150.0,
            30..=59 => 300.0,
            _ => 1500.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fresh_player_has_tight_skill_range() {
        let player = Player::new(1, 1500.0);
        assert_eq!(player.acceptable_skill_range(), 50.0);
    }

    #[test]
    fn test_skill_rating_stored_correctly() {
        let player = Player::new(42, 2000.5);
        assert_eq!(player.id, 42);
        assert_eq!((player.skill_rating - 2000.5).abs() < 0.001); 
    }
}
