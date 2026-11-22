use super::rating::{Change, Encounter, Odds, Player, Rule, settle};

fn game(
    a: &mut Player,
    b: &mut Player,
    ids: [usize; 2],
    hour: u32,
    repeat: usize,
    won: bool,
    odds: &Odds,
) -> Change {
    settle(
        [a, b],
        Encounter {
            ids,
            now: 3600 * hour,
            repeats: repeat,
            a_won: won,
        },
        Rule::Evidence,
        odds,
    )
}
/// Condition the example on completed matches against different established
/// opponents. Keep both current scores far enough apart for the full quote.
pub(super) fn quote(
    low_run: u8,
    high_run: u8,
    rookie: bool,
    repeats: usize,
    odds: &Odds,
) -> [Change; 2] {
    let mut low = Player::at(1300, if rookie { 0 } else { 100 });
    let mut high = Player::at(2200, 100);
    for i in 0..low_run {
        game(
            &mut low,
            &mut Player::at(2200, 100),
            [0, 10 + usize::from(i)],
            u32::from(i),
            0,
            true,
            odds,
        );
    }
    for i in 0..high_run {
        game(
            &mut high,
            &mut Player::at(1300, 100),
            [1, 20 + usize::from(i)],
            u32::from(i),
            0,
            false,
            odds,
        );
    }
    [true, false].map(|won| {
        game(
            &mut low.clone(),
            &mut high.clone(),
            [0, 1],
            5,
            repeats,
            won,
            odds,
        )
    })
}
pub(super) fn run() {
    let odds = Odds::new();
    println!(
        "prior_low_wins,prior_high_losses,low_in_placement,prior_pair_games_6h,upset_low,upset_high,expected_low,expected_high"
    );
    for lw in 0..=3 {
        for hl in 0..=3 {
            for rookie in [false, true] {
                for repeats in [0, 2, 4] {
                    let [upset, expected] = quote(lw, hl, rookie, repeats, &odds);
                    println!(
                        "{lw},{hl},{rookie},{repeats},{:.2},{:.2},{:.2},{:.2}",
                        upset.delta[0], upset.delta[1], expected.delta[0], expected.delta[1]
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn two_independent_histories_and_placement_protection() {
        let odds = Odds::new();
        for a in 0..=3 {
            for b in 0..=3 {
                for rookie in [false, true] {
                    for (repeats, weight) in [(0, 1.0), (2, 0.5), (4, 0.25)] {
                        let [surprise, normal] = quote(a, b, rookie, repeats, &odds);
                        assert_eq!(surprise.streak, [a, b]);
                        assert_eq!(
                            surprise.delta,
                            [
                                weight * (20.0 + 4.0 * f64::from(a)),
                                -weight
                                    * if rookie {
                                        10.0
                                    } else {
                                        14.0 + 4.0 * f64::from(b)
                                    }
                            ]
                        );
                        assert!(normal.delta[0] < 0.0 && normal.delta[1] > 0.0);
                    }
                }
            }
        }
    }
    #[test]
    fn eligible_normal_result_clears_a_run_and_stale_results_expire() {
        let odds = Odds::new();
        let mut low = Player::at(1200, 100);
        for i in 0..3 {
            game(
                &mut low,
                &mut Player::at(2100, 100),
                [0, 10 + i],
                i as u32,
                0,
                true,
                &odds,
            );
        }
        let mut old = low.clone();
        let old_win = game(
            &mut old,
            &mut Player::at(2100, 100),
            [0, 50],
            360,
            0,
            true,
            &odds,
        );
        assert_eq!(old_win.streak[0], 0);
        game(
            &mut low,
            &mut Player::at(2100, 100),
            [0, 44],
            4,
            0,
            false,
            &odds,
        );
        assert_eq!(
            game(
                &mut low,
                &mut Player::at(2100, 100),
                [0, 47],
                5,
                0,
                true,
                &odds
            )
            .streak[0],
            0
        );
    }
    #[test]
    fn a_repeat_opponent_cannot_build_both_sides_a_streak() {
        let odds = Odds::new();
        let mut low = Player::at(1050, 100);
        let mut high = Player::at(2450, 100);
        for i in 0..10 {
            let result = game(&mut low, &mut high, [0, 1], i * 8, 0, true, &odds);
            assert_eq!(result.streak, [0, 0]);
            assert_eq!(result.delta, [20.0, -14.0]);
        }
    }
}
