use crate::player::Player;

#[derive(Debug)]
pub struct GameMatch {
    pub id: u64,                //Unique match ID
    pub team_a: Vec<Player>,    //Players on Team A
    pub team_b: Vec<Player>,    //Players on Team B
    pub quality_score: f64,     //How good is this match? Higher is better (0.0 to 1.0)
    pub avg_wait_time: f64,     //Average wait time of all players in this match (in seconds)   
}

impl GameMatch {
    pub fn new(id: u64, team_a: Vec<Player>, team_b: Vec<Player>) -> Self {
        let quality_score = calculate_quality(&team_a, &team_b);
        
        let avg_wait_ms = team_a.iter()
            .chain(team_b.iter())           //iterate all 10 players
            .map(|p| p.wait_ms())           //convert to milliseconds
            .sum::<u64>() as f64            //sum them up and convert to f64
            / 10.0;                         //divide by player count 
        
        GameMatch {
            id,
            team_a,
            team_b,
            quality_score,
            avg_wait_ms,
        }
    }
}

// Quality Scoring: 
// Two components:
//  1. BALANCE SCORE (weight: 70%)
//     How close are the two team averages?
//     Example: Team A avg = 1500, Team B avg = 1480 → nearly perfect (0.99)
//              Team A avg = 1500, Team B avg = 1300 → unbalanced (0.00)
//
//  2. SPREAD SCORE (weight: 30%)
//     How uniform is skill WITHIN each team?
//     A team with players [1400, 1450, 1500, 1550, 1600] is better than
//     a team with players [800, 1000, 1500, 2000, 2200] — same average,
//     but the second team has huge internal mismatch.

fn compute_quality(team_a: &[Player], team_b: &[Player]) -> f64 {
    let avg_a = mean(team_a);
    let avg_b = mean(team_b);

    let balance_diff = (avg_a - avg_b).abs();
    let balance_score = (1.0 - (balance_diff / 200.0)).max(0.0); 

    let spread_a = std_dev(team_a, avg_a);
    let spread_b = std_dev(team_b, avg_b);
    let spread_score = 1.0 - ((spread_a + spread_b) / 2.0 / 300.0).max(0.0);

    balance_score * 0.70 + spread_score * 0.30
}

fn mean(players: &[Player]) -> f64 {
    players.iter().map(|p| p.skill_rating).sum::<f64>() / players.len() as f64
}

fn std_dev(players: &[Player], mean: f64) -> f64 {
    let variance = players.iter()
        .map(|p| (p.skill_rating - mean).powi(2))
        .sum::<f64>() 
        / players.len() as f64;
    variance.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::Player;

    fn make_team(rating: &[f64]) -> Vec<Player> {
        rating.iter().enumerate()
            .map(|(i, &r)| Player::new(i as u64, r))
            .collect()
    }

    #[test]
    fn test_perfect_balance_gets_high_score() {
        let a = make_team(&[1480.0, 1490.0, 1500.0, 1510.0, 1520.0]);
        let b = make_team(&[1480.0, 1490.0, 1500.0, 1510.0, 1520.0]);
        
        let game_match = GameMatch::new(1, a, b);

        assert!(game_match.quality_score > 0.90, "Expected high quality score, got {}", game_match.quality_score);
    }

    #[test]
    fn test_unbalanced_teams_get_low_score() {
        let a = make_team(&[500.0, 520.0, 510.0, 530.0, 515.0]);
        let b = make_team(&[2500.0, 2480.0, 2520.0, 2510.0, 2495.0]);

        let game_match = GameMatch::new(2, a, b);

        assert!(game_match.quality_score < 0.40, "Expected low quality score, got {}", game_match.quality_score);
    }
}