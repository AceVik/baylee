//! `cards/creatures/mv_3/elvish_spirit_guide.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn elvish_spirit_guide() -> CardIndex {
    card_index("6b0e23cf-7d68-4329-86db-7adc26abd86b")
}

/// `Elvish Spirit Guide` ({2}{G}, 2/2 Creature — Elf Spirit) exiles itself from
/// your hand as a mana ability to add `{G}`.
#[test]
fn elvish_spirit_guide_exiles_from_hand_for_green_mana_with_no_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[elvish_spirit_guide(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let guide = in_hand(&engine, p0, elvish_spirit_guide()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: guide,
                ability_index: 0,
            },
        )
        .unwrap();

    assert_eq!(engine.state().object(guide).unwrap().zone, Zone::Exile);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );

    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_some());
}

/// "Exile this card from your hand": the ability is a hand ability. A Guide on the battlefield
/// cannot be exiled for mana.
#[test]
fn elvish_spirit_guide_cannot_be_exiled_for_mana_from_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[elvish_spirit_guide()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let guide = on_battlefield(&engine, p0, elvish_spirit_guide()).unwrap();
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: guide,
                    ability_index: 0,
                },
            )
            .is_err(),
        "the ability works from the hand only"
    );
    assert_eq!(
        engine.state().object(guide).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        0
    );
}
