//! `cards/instants/mv_3/tidal_wave.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn tidal_wave() -> CardIndex {
    card_index("c5f5b7e5-af1e-4c8a-8d44-3e2152955d4e")
}

/// Tidal Wave (`{2}{U}`, `Coverage::Partial`): creates a 5/5 blue Wall creature
/// token with defender.
#[test]
fn tidal_wave_creates_5_5_blue_wall_token_with_defender() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(); 3])
        .hand(0, &[tidal_wave()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, tidal_wave());
    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1);
    assert_eq!(pt(&engine, tokens[0]), (5, 5));
    assert!(keywords(&engine, tokens[0]).contains(KeywordSet::DEFENDER));
}
