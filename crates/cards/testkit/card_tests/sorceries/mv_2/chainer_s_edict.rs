//! `cards/sorceries/mv_2/chainer_s_edict.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

fn chainer_s_edict() -> CardIndex {
    card_index("7975cb9d-d37e-44f8-9614-86d2ad453c35")
}

#[track_caller]
fn start_flashback(engine: &mut Engine<RegistryLookup>, player: PlayerId, edict: ObjectId) {
    engine
        .apply(player, PlayerAction::CastSpell { card: edict })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let flashback = choose_cast_kind(engine, CastModeKind::Flashback);
        engine
            .apply(player, PlayerAction::ChooseMode(flashback))
            .unwrap();
    }
}

/// Resolve an Edict already aimed at `target`, who chooses exactly one of their
/// own creatures. Return the complete sacrifice offer for the caller's asserts.
#[track_caller]
fn choose_sacrifice(
    engine: &mut Engine<RegistryLookup>,
    target: PlayerId,
    victim: ObjectId,
) -> Vec<ObjectId> {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == target),
    );
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!(player, target);
    assert_eq!((min, max), (1, 1), "sacrifice exactly one creature");
    assert!(options.contains(&victim), "{victim:?} is not offered");
    engine
        .apply(
            target,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
    options
}

/// The targeted opponent, not the caster, chooses which of their creatures to
/// sacrifice. The flashback costs all seven mana of {5}{B}{B}, repeats that
/// choice on the remaining creature and exiles the spell.
#[test]
fn chainer_s_edict_target_player_chooses_twice_and_flashback_exiles() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = vec![swamp(); 9];
    board.push(index::HILL_GIANT);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &board)
        .battlefield(1, &[index::GRIZZLY_BEARS, index::GRAY_OGRE])
        .hand(0, &[chainer_s_edict()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let own_giant = on_battlefield(&engine, p0, index::HILL_GIANT).unwrap();
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    let ogre = on_battlefield(&engine, p1, index::GRAY_OGRE).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, chainer_s_edict());
    let Pending::ChoosePlayer { player, options } = engine.pending().clone() else {
        panic!("expected a player target, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert!(options.contains(&p0) && options.contains(&p1));
    assert_eq!(options.len(), 2, "exactly the two players are offered");
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    assert_eq!(engine.state().players[0].mana_pool.total(), 7);

    let first_options = choose_sacrifice(&mut engine, p1, bears);
    assert!(first_options.contains(&bears) && first_options.contains(&ogre));
    assert!(
        !first_options.contains(&own_giant),
        "the caster's creature is not the opponent's choice"
    );
    assert_eq!(engine.state().object(bears).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().object(ogre).unwrap().zone, Zone::Battlefield);
    assert_eq!(
        engine.state().object(own_giant).unwrap().zone,
        Zone::Battlefield
    );
    let edict = in_graveyard(&engine, p0, chainer_s_edict()).unwrap();

    start_flashback(&mut engine, p0, edict);
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(engine.state().object(edict).unwrap().zone, Zone::Stack);
    let second_options = choose_sacrifice(&mut engine, p1, ogre);
    assert_eq!(second_options, vec![ogre]);

    assert_eq!(engine.state().object(ogre).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        engine.state().object(own_giant).unwrap().zone,
        Zone::Battlefield,
        "neither casting sacrifices a creature the target does not control"
    );
    assert_eq!(engine.state().object(edict).unwrap().zone, Zone::Exile);
    assert!(in_graveyard(&engine, p0, chainer_s_edict()).is_none());
}

/// Six black mana are one short of {5}{B}{B}; the ordinary {1}{B} cost cannot
/// make the graveyard spell payable.
#[test]
fn chainer_s_edict_flashback_is_not_offered_with_only_six_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(); 6])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[chainer_s_edict()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let edict = hand_to_graveyard(&mut engine, p0, chainer_s_edict());
    tap_all_mana(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(!legal.castable.contains(&edict));
    assert!(!legal.payable.contains(&edict));
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: edict })
            .is_err()
    );
    assert_eq!(engine.state().object(edict).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().players[0].mana_pool.total(), 6);
}
