//! `cards/creatures/mv_3/dusk_imp.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dusk Imp is the whole card in its cost and its keyword: `{2}{B}` for a 2/1
/// Imp with flying. Casting it spends exactly the three Swamps the Elf beside
/// them is kept out of, and the body is read off the *projection*, so an Imp
/// that arrives but never carries its keyword cannot pass. The combat step a
/// turn later — needed because a creature cast this turn may not attack
/// (CR 302.6) — is what makes flying an effect rather than a characteristic:
/// both creatures attack, and the defender's printed 1/1 Elf is offered as a
/// blocker for the ground attacker while the flier is declined (CR 702.9a).
#[test]
fn dusk_imp_lands_as_a_flying_two_one_that_a_ground_creature_cannot_block() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[dusk_imp()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {2}{B} off the three Swamps. The Elf is named as the thing kept back
    // because it is the creature that has to attack beside the Imp next turn.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        3,
        "three Swamps, and the Elf beside them still standing"
    );
    cast_with_floating(&mut engine, p0, dusk_imp());
    pass_until(&mut engine, stack_is_empty);

    let imp = on_battlefield(&engine, p0, dusk_imp()).expect("the Imp resolved");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(pt(&engine, imp), (2, 1), "the printed 2/1 body");
    assert!(
        types(&engine, imp).contains(TypeSet::CREATURE),
        "and the type line it was played for"
    );
    assert!(
        keywords(&engine, imp).contains(KeywordSet::FLYING),
        "`flying` reaches the permanent through the layers"
    );

    // A full turn cycle: the Imp was cast this turn and a creature that has
    // not been under its controller's control since their turn began cannot
    // attack (CR 302.6) or use a {T} ability.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(imp, Defender::Player(p1)), (mine, Defender::Player(p1))],
            },
        )
        .expect("both are untapped and have been controlled since the turn began");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    let Pending::ChooseBlockers {
        player, blockers, ..
    } = engine.pending().clone()
    else {
        panic!(
            "the defending seat is asked which of its creatures block, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the seat being attacked declares the blocks");
    let ground = blockers
        .iter()
        .find(|option| option.blocker == theirs)
        .expect("the Elf across the table may block something");
    assert!(
        ground.attackers.contains(&mine),
        "the ground attacker is one it may block, so the list is not empty for \
         the wrong reason: {:?}",
        ground.attackers
    );
    assert!(
        !ground.attackers.contains(&imp),
        "`flying` is the whole of the Imp's text, and a creature with neither \
         flying nor reach cannot block it (CR 702.9a): {:?}",
        ground.attackers
    );
}
