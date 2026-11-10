//! `cards/lands/utility/haven_of_the_spirit_dragon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Haven of the Spirit Dragon prints `{{T}}: Add {{C}}.`, `{{T}}: Add one mana of any color. Spend this mana only to cast a Dragon creature spell.`, and `{{2}}, {{T}}, Sacrifice this land: Return target Dragon creature card or Ugin planeswalker card from your graveyard to your hand.`
///
/// Under `Coverage::Implemented`, activating ability 1 prompts for a color choice and produces restricted mana spendable only on Dragon creature spells.
/// Because this mana carries a spending restriction, it appears in `pool.restricted()` rather than `pool.available()`.
/// With an empty graveyard, ability 2 is withheld from `legal.abilities` for lack of a legal target.
#[test]
fn haven_of_the_spirit_dragon_produces_restricted_dragon_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[haven_of_the_spirit_dragon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let haven = on_battlefield(&engine, p0, haven_of_the_spirit_dragon())
        .expect("haven of the spirit dragon on battlefield");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(haven, 0)),
        "ability 0 (colorless mana) is offered"
    );
    assert!(
        legal.abilities.contains(&(haven, 1)),
        "ability 1 (restricted dragon mana) is offered"
    );
    assert!(
        !legal.abilities.contains(&(haven, 2)),
        "ability 2 is not offered when the graveyard has no target"
    );

    activate(&mut engine, p0, haven_of_the_spirit_dragon(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "restricted mana does not appear in simple available pool"
    );
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Red);
    assert!(is_tapped(&engine, haven));
}

/// Haven of the Spirit Dragon: "{2}, {T}, Sacrifice this land: Return target
/// Dragon creature card or Ugin planeswalker card from your graveyard to your
/// hand." Shivan Dragon is on offer, Llanowar Elves is not.
#[test]
fn haven_of_the_spirit_dragon_returns_a_dragon_card_to_hand() {
    let p0 = PlayerId::new(0);
    let haven = card_index("acc9c16a-5e72-43bd-87e1-56a16aa892f5");
    let dragon = card_index("711eea87-0fa3-46e0-a42b-fa5a86455f04");
    let mut engine = Duel::new(2303, forest())
        .battlefield(0, &[haven, forest(), forest(), dragon, llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let h = on_battlefield(&engine, p0, haven).expect("haven");
    let d = on_battlefield(&engine, p0, dragon).expect("dragon");
    let e = on_battlefield(&engine, p0, llanowar_elves()).expect("elf");
    bury(&mut engine, &[d, e]);
    tap_mana_where(&mut engine, p0, |id| id != h && id != d && id != e);

    activate(&mut engine, p0, haven, 2);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(options.contains(&d) && !options.contains(&e));
    unf_aim_and_pay(&mut engine, p0, Some(d), None);

    assert!(in_hand(&engine, p0, dragon).is_some());
    assert!(in_graveyard(&engine, p0, haven).is_some(), "sacrificed");
}
