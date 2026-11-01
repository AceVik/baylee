//! `cards/creatures/artifacts/mv_5/urza_s_engine.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

#[test]
fn urza_s_engine_gains_banding_and_grants_trample_to_banded_attackers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("bf6ea878-044b-40a8-a21b-0191a948a69a");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                card,
                GRIZZLY_BEARS,
                HILL_GIANT,
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let juggernaut = on_battlefield(&engine, p0, card).unwrap();
    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();

    assert!(
        engine
            .state()
            .object(juggernaut)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::TRAMPLE)
    );
    assert!(
        !engine
            .state()
            .object(juggernaut)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::BANDING)
    );

    // Ability 0 ({3}): Urza's Engine gains banding until end of turn.
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, card, 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .object(juggernaut)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::BANDING)
    );

    // Attack with Urza's Engine, Grizzly Bears (banded with it), and Hill Giant (outside the band).
    declare_band_attack(&mut engine, p0, p1, &[juggernaut, bear, giant]);
    answer_band_and_reach_blockers(&mut engine, p0, vec![bear]);

    // Ability 1 ({3}): Attacking creatures banded with Urza's Engine gain trample until end of turn.
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    engine.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Green, 3);
    engine.refresh_offer();
    activate(&mut engine, p0, card, 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(bear)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::TRAMPLE),
        "Grizzly Bears is banded with Urza's Engine and should gain trample"
    );
    assert!(
        !engine
            .state()
            .object(giant)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::TRAMPLE),
        "Hill Giant is not banded with Urza's Engine and should not gain trample"
    );
}
