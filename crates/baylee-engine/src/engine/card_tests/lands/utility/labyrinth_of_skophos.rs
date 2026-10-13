//! `cards/lands/utility/labyrinth_of_skophos.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The land produces colorless mana. Its combat ability needs an attacker or
/// blocker even when its mana cost is affordable.
#[test]
fn labyrinth_needs_a_combat_target_even_when_its_cost_is_affordable() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(127, forest())
        // Four Forests pay the combat ability’s {4} — generic mana
        // takes any colour — because the pin below is read off the offer and
        // `can_afford` reads the pool.
        .battlefield(
            0,
            &[
                labyrinth_of_skophos(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lab = on_battlefield(&engine, p0, labyrinth_of_skophos()).expect("Labyrinth deployed");

    // Read **before** the mana ability is pressed and with the price
    // floating. It used to be read after, on an empty pool and a tapped
    // land, so two separate things guaranteed the absence and neither of
    // them was the card.
    tap_mana_except(&mut engine, p0, lab);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the {{4}} is standing in the pool"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(s, ai)| *s == lab && *ai == 1),
        "no second ability is offered, and its {{4}} is paid for right here: \
         {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, labyrinth_of_skophos(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, lab));
}

#[test]
fn labyrinth_removes_attackers_or_blockers_without_unblocking_or_untapping() {
    use crate::engine::card_tests::legends_kit as kit;
    for remove_blocker in [false, true] {
        let mut engine = kit::game(
            944,
            &[labyrinth_of_skophos(), llanowar_elves()],
            &[llanowar_elves()],
            &[],
            &[],
        );
        let attacker = kit::obj(&engine, kit::P0, llanowar_elves());
        let blocker = kit::obj(&engine, kit::P1, llanowar_elves());
        kit::attackers_offered(&mut engine);
        kit::declare(&mut engine, &[attacker]);
        kit::block(&mut engine, &[(blocker, attacker)]);
        kit::float(&mut engine, kit::P0, &[(ManaColor::Colorless, 4)]);
        kit::use_ability(
            &mut engine,
            kit::P0,
            labyrinth_of_skophos(),
            1,
            &[if remove_blocker { blocker } else { attacker }],
            &[],
        );
        assert!(is_tapped(&engine, attacker));
        let life = kit::life(&engine, kit::P1);
        pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
        assert_eq!(kit::life(&engine, kit::P1), life);
        assert_eq!(
            engine.state().object(attacker).unwrap().zone,
            Zone::Battlefield
        );
        assert_eq!(
            engine.state().object(blocker).unwrap().zone,
            Zone::Battlefield
        );
    }
}

/// "Remove target attacking or blocking creature from combat": an idle
/// creature on either side is not a legal target, and an unblocked attacker
/// that is removed deals no combat damage.
#[test]
fn labyrinth_offers_only_combatants_and_a_removed_attacker_deals_no_damage() {
    use crate::engine::card_tests::legends_kit as kit;
    use baylee_core::generated::index;
    let mut engine = kit::game(
        946,
        &[
            labyrinth_of_skophos(),
            index::GRIZZLY_BEARS,
            llanowar_elves(),
        ],
        &[serra_angel()],
        &[],
        &[],
    );
    let attacker = kit::obj(&engine, kit::P0, index::GRIZZLY_BEARS);
    let idle_mine = kit::obj(&engine, kit::P0, llanowar_elves());
    let idle_theirs = kit::obj(&engine, kit::P1, serra_angel());
    kit::attack(&mut engine, &[attacker]);
    kit::block(&mut engine, &[]);
    kit::float(&mut engine, kit::P0, &[(ManaColor::Colorless, 4)]);
    activate(&mut engine, kit::P0, labyrinth_of_skophos(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending());
    };
    assert_eq!(
        options,
        vec![attacker],
        "\"attacking or blocking\": only the attacker, not the idle creatures \
         {idle_mine:?} / {idle_theirs:?}"
    );
    let life = kit::life(&engine, kit::P1);
    kit::aim(&mut engine, &[attacker], &[]);
    kit::settle(&mut engine);
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        kit::life(&engine, kit::P1),
        life,
        "\"remove from combat\": the removed attacker deals no combat damage"
    );
    assert!(
        is_tapped(&engine, attacker),
        "removal from combat does not untap it"
    );
}
