//! `cards/creatures/mv_3/mistform_wall.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;
use baylee_core::generated::subtypes;

fn mistform_wall() -> CardIndex {
    card_index("02c4abd0-9a68-4a85-954b-3f64666c1f8d")
}

/// Mistform Wall has defender while it is a Wall and cannot attack; activating
/// its `{1}` ability to become a non-Wall removes defender so it can attack,
/// and cleanup restores Wall and defender.
#[test]
fn mistform_wall_loses_defender_and_attacks_when_no_longer_a_wall() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), mistform_wall(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wall = on_battlefield(&engine, p0, mistform_wall()).expect("Mistform Wall seated");
    assert!(keywords(&engine, wall).contains(KeywordSet::DEFENDER));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, mistform_wall(), 1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::BIRD))
        .expect("choose Bird");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        !keywords(&engine, wall).contains(KeywordSet::DEFENDER),
        "no longer a Wall, so defender disappears"
    );

    let attackers = attackers_offered(&mut engine);
    assert!(attackers.contains(&wall), "Bird Mistform Wall may attack");
    declare(&mut engine, &[wall]);
    through_combat(&mut engine);
    assert_eq!(
        life(&engine, PlayerId::new(1)),
        19,
        "dealt 1 combat damage to defender"
    );

    // Next turn reverts to Wall and regains defender; Llanowar Elves keeps
    // DeclareAttackers offered so we observe `wall` omitted from the menu.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(keywords(&engine, wall).contains(KeywordSet::DEFENDER));
    let attackers_next = attackers_offered(&mut engine);
    assert!(
        !attackers_next.contains(&wall),
        "Wall Mistform Wall regains defender and cannot attack"
    );
}
