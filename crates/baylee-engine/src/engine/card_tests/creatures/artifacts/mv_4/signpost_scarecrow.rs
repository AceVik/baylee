//! `cards/creatures/artifacts/mv_4/signpost_scarecrow.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn signpost_scarecrow() -> CardIndex {
    card_index("5b6c6dab-0687-4297-b215-89e699d65681")
}

/// Signpost Scarecrow — {4} 2/4 with vigilance and "{2}: Add one mana of any
/// color." Attacking leaves it untapped; two floating Forest mana turn into
/// one mana of the colour asked for.
#[test]
fn signpost_scarecrow_filters_two_into_one_of_any_color() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[signpost_scarecrow(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let scarecrow = on_battlefield(&engine, p0, signpost_scarecrow()).expect("seated");
    assert!(keywords(&engine, scarecrow).contains(KeywordSet::VIGILANCE));

    let forests = all_on_battlefield(&engine, p0, forest());
    for land in &forests {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: *land })
            .expect("a Forest taps for mana");
    }
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        2
    );
    activate(&mut engine, p0, signpost_scarecrow(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("which colour is asked: {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5, "any of the five");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue is on the menu");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert_eq!(pool.available(ManaColor::Green), 0, "the {{2}} was paid");
    assert_eq!(pool.total(), 1);

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(scarecrow, Defender::Player(p1))],
            },
        )
        .expect("it attacks");
    assert!(!is_tapped(&engine, scarecrow), "vigilance");
}
