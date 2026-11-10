//! `cards/lands/utility/ally_encampment.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ally Encampment is a utility land under `Coverage::Implemented` that produces {C} or restricted mana of any color for Ally spells.
/// Playing the land enters the battlefield untapped.
/// Activating its second mana ability prompts for a color choice via `Pending::ChooseColor` and adds restricted mana to the pool.
/// Following the restricted mana rules, the mana appears in `pool.restricted()` rather than general available mana.
#[test]
fn ally_encampment_adds_restricted_mana_of_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[ally_encampment()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, ally_encampment());
    assert!(
        !entered_tapped(&engine, land),
        "Ally Encampment enters untapped"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 1,
            },
        )
        .expect("activating restricted mana ability is legal");

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending())
    };
    assert_eq!(options.len(), 5, "offers all five colors");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("choosing White is legal");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        0,
        "restricted mana does not appear in general available pool"
    );
    assert_eq!(
        pool.restricted().len(),
        1,
        "one restricted mana entry recorded"
    );
    assert_eq!(pool.restricted()[0].amount, 1, "exactly one mana added");
    assert_eq!(
        pool.restricted()[0].color,
        ManaColor::White,
        "restricted mana color matches chosen color"
    );
    assert!(
        is_tapped(&engine, land),
        "Ally Encampment tapped to produce restricted mana"
    );
}

/// Ally Encampment: "{1}, {T}, Sacrifice this land: Return target Ally you
/// control to its owner's hand." Ondu Cleric is an Ally; the Elves are not.
#[test]
fn ally_encampment_returns_an_ally_to_hand() {
    let p0 = PlayerId::new(0);
    let camp = card_index("9d293b69-12b7-4b50-a0a7-c4f493dee30b");
    let mut engine = Duel::new(2304, forest())
        .battlefield(0, &[camp, forest(), ondu_cleric(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let c = on_battlefield(&engine, p0, camp).expect("camp");
    let cleric = on_battlefield(&engine, p0, ondu_cleric()).expect("cleric");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf");
    tap_mana_where(&mut engine, p0, |id| id != c && id != cleric && id != elf);

    activate(&mut engine, p0, camp, 2);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(options.contains(&cleric) && !options.contains(&elf));
    unf_aim_and_pay(&mut engine, p0, Some(cleric), None);

    assert!(in_hand(&engine, p0, ondu_cleric()).is_some());
    assert!(on_battlefield(&engine, p0, ondu_cleric()).is_none());
    assert!(in_graveyard(&engine, p0, camp).is_some(), "sacrificed");
}
