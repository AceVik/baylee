//! `cards/enchantments/mv_4/spiritual_asylum.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Spiritual Asylum — {2}{W}{W} — Enchantment: "Creatures and lands you
/// control have shroud" and "Whenever a creature you control attacks,
/// sacrifice this enchantment."
///
/// Both printed sentences are played on one board, since each is the other's
/// control. Shroud is read off Vindicate's published target menu, which is the
/// only place a targeting restriction is visible: the same menu offers the
/// opponent's own Elf and Plains — so the list is enumerated and its scoping
/// is real — while declining this seat's Forest and Elves, the two card types
/// the static names. The trigger is then played, and the enchantment leaving
/// for its owner's graveyard on an attack is the whole of the second sentence.
#[test]
fn spiritual_asylum_shrouds_your_creatures_and_lands_and_is_sacrificed_to_an_attack() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[spiritual_asylum(), forest(), llanowar_elves()])
        .battlefield(1, &[plains(), plains(), swamp(), llanowar_elves()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let my_forest = on_battlefield(&engine, p0, forest()).expect("my Forest is out");
    let asylum = on_battlefield(&engine, p0, spiritual_asylum()).expect("the Asylum is out");
    let their_elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let their_land = on_battlefield(&engine, p1, plains()).expect("their Plains are out");

    // p1 casts Vindicate — "destroy target permanent" — on their own main
    // phase, where every permanent on the table but the shrouded pair is a
    // legal target.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until only stops on a target choice")
    };
    assert_eq!(player, p1, "the seat that cast it names the target");
    assert!(
        options.contains(&their_elves) && options.contains(&their_land),
        "their own Elf and their own Plains are on the menu, so the list is \
         enumerated and \"you control\" is what is being read: {options:?}"
    );
    assert!(
        options.contains(&asylum),
        "the Asylum is an enchantment, and the static shrouds creatures and \
         lands rather than the source itself: {options:?}"
    );
    assert!(
        !options.contains(&my_forest),
        "\"lands you control have shroud\": my Forest cannot be the target of \
         their spell: {options:?}"
    );
    assert!(
        !options.contains(&elves),
        "\"creatures you control have shroud\": my Elves cannot be targeted \
         either — the same card stands across the table and is offered there, \
         so the exclusion is the shroud and not the filter: {options:?}"
    );

    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![their_elves],
            },
        )
        .expect("a creature the question offered is a legal answer");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the Vindicate resolved against the target it was given, so the menu \
         above was an honest offer rather than a list nothing could come of"
    );

    // Back to p0, whose Elves are untapped and past summoning sickness.
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, spiritual_asylum()).is_some(),
        "nothing has attacked yet, so the Asylum is still standing"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing else here")
    };
    assert_eq!(player, p0, "it is my combat step");
    assert!(
        attackers.contains(&elves),
        "an untapped 1/1 under my control may attack: {attackers:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elves, Defender::Player(p1))],
            },
        )
        .expect("the attacker came out of the list that offered it");
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, spiritual_asylum()).is_none()
    });

    assert!(
        in_graveyard(&engine, p0, spiritual_asylum()).is_some(),
        "\"whenever a creature you control attacks, sacrifice this \
         enchantment\" — a sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the price was the enchantment and not the creature that paid it"
    );
}
