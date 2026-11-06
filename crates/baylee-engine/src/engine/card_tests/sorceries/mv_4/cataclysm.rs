//! `cards/sorceries/mv_4/cataclysm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

fn cataclysm() -> CardIndex {
    card_index("b0479ca4-e925-4aa7-a4fa-f7e01f899ac5")
}

fn pass_to_keep_choice(engine: &mut Engine<RegistryLookup>) {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::KeepPermanents,
                ..
            }
        )
    });
}

fn keep_choice(engine: &Engine<RegistryLookup>) -> (PlayerId, Vec<ObjectId>) {
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected a Cataclysm keep choice, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(prompt, ChoicePrompt::KeepPermanents);
    assert_eq!((min, max), (1, 1));
    (player, options)
}

fn keep(engine: &mut Engine<RegistryLookup>, player: PlayerId, object: ObjectId) {
    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: vec![object],
            },
        )
        .unwrap();
}

fn kept_announcements(engine: &Engine<RegistryLookup>, from: usize) -> Vec<PlayerId> {
    engine.state().journal.entries()[from..]
        .iter()
        .filter_map(|entry| {
            if let GameEvent::CardsKept { player, .. } = entry.event {
                Some(player)
            } else {
                None
            }
        })
        .collect()
}

/// One artifact creature may cover both printed choices. A planeswalker is
/// none of the four named kinds and therefore cannot be chosen to survive.
#[test]
fn cataclysm_reuses_a_multi_type_permanent_and_sacrifices_unlisted_types() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                ornithopter(),
                index::GRIZZLY_BEARS,
                rhystic_study(),
                index::JACE_THE_MIND_SCULPTOR,
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .hand(0, &[cataclysm()])
        .start();
    let jace = on_battlefield(&engine, p0, index::JACE_THE_MIND_SCULPTOR).unwrap();
    engine
        .dev_state_mut(p0)
        .unwrap()
        .object_mut(jace)
        .unwrap()
        .counters
        .set(CounterKind::Loyalty, 4);
    engine.refresh_offer();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let thopter = on_battlefield(&engine, p0, ornithopter()).unwrap();
    let bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();
    let study = on_battlefield(&engine, p0, rhystic_study()).unwrap();
    let land = on_battlefield(&engine, p0, plains()).unwrap();
    cast_from_hand(&mut engine, p0, cataclysm());
    assert!(on_stack(&engine, cataclysm()).is_some());

    // The only artifact is retained automatically. It remains a legal creature.
    pass_to_keep_choice(&mut engine);
    let (chooser, options) = keep_choice(&engine);
    assert_eq!(chooser, p0);
    assert_eq!(options, vec![thopter, bears]);
    assert!(!options.contains(&jace));
    keep(&mut engine, p0, thopter);

    // Rhystic Study is the only enchantment. One of four Plains is a real choice.
    pass_to_keep_choice(&mut engine);
    let (chooser, options) = keep_choice(&engine);
    assert_eq!(chooser, p0);
    assert_eq!(options.len(), 4);
    assert!(options.iter().all(|id| lands_of(&engine, p0).contains(id)));
    keep(&mut engine, p0, land);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(thopter).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state().object(study).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(engine.state().object(land).unwrap().zone, Zone::Battlefield);
    assert_eq!(engine.state().object(bears).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().object(jace).unwrap().zone, Zone::Graveyard);
    assert_eq!(lands_of(&engine, p0).len(), 1);
    assert!(in_graveyard(&engine, p0, cataclysm()).is_some());
}

/// Choosing an artifact creature as the artifact does not consume the creature
/// choice. Illegal answers cannot change either the board or the continuation.
#[test]
fn cataclysm_accepts_only_an_offered_permanent_for_each_choice() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                ornithopter(),
                index::MEMNITE,
                sol_ring(),
                index::GRIZZLY_BEARS,
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .battlefield(1, &[index::MEMNITE])
        .hand(0, &[cataclysm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let thopter = on_battlefield(&engine, p0, ornithopter()).unwrap();
    let memnite = on_battlefield(&engine, p0, index::MEMNITE).unwrap();
    let ring = on_battlefield(&engine, p0, sol_ring()).unwrap();
    let bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();
    let foreign = on_battlefield(&engine, p1, index::MEMNITE).unwrap();
    cast_from_hand(&mut engine, p0, cataclysm());
    pass_to_keep_choice(&mut engine);
    keep(&mut engine, p0, thopter);
    pass_to_keep_choice(&mut engine);

    for (actor, objects) in [
        (p0, vec![]),
        (p0, vec![foreign]),
        (p0, vec![ring]),
        (p1, vec![bears]),
    ] {
        let state = engine.state().snapshot_hash();
        let continuation = engine.snapshot_hash();
        assert!(
            engine
                .apply(actor, PlayerAction::ChooseObjects { objects })
                .is_err()
        );
        assert_eq!(engine.state().snapshot_hash(), state);
        assert_eq!(engine.snapshot_hash(), continuation);
    }
    keep(&mut engine, p0, bears);
    pass_to_keep_choice(&mut engine);
    let land = on_battlefield(&engine, p0, plains()).unwrap();
    keep(&mut engine, p0, land);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(thopter).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state().object(bears).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state().object(foreign).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state().object(memnite).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(engine.state().object(ring).unwrap().zone, Zone::Graveyard);
    assert_eq!(lands_of(&engine, p0).len(), 1);
}

fn three_player_cataclysm() -> Engine<RegistryLookup> {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[sol_ring(), sol_ring()])
        .battlefield(1, &[sol_ring(), sol_ring()])
        .battlefield(2, &[sol_ring(), sol_ring()])
        .hand(1, &[cataclysm()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    {
        let state = engine.dev_state_mut(p1).unwrap();
        state.players[1].mana_pool.add(ManaColor::White, 2);
        state.players[1].mana_pool.add(ManaColor::Colorless, 2);
    }
    engine.refresh_offer();
    cast_with_floating(&mut engine, p1, cataclysm());
    pass_to_keep_choice(&mut engine);
    engine
}

/// Choices follow APNAP from the active player, but every unchosen permanent
/// stays until all three players have chosen; then the sacrifices are simultaneous.
#[test]
fn cataclysm_three_players_choose_in_apnap_before_anything_is_sacrificed() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut engine = three_player_cataclysm();
    let journal_from = engine.state().journal.len();
    let rings: Vec<_> = [p0, p1, p2]
        .into_iter()
        .flat_map(|player| all_on_battlefield(&engine, player, sol_ring()))
        .collect();
    assert_eq!(rings.len(), 6);

    for (index, player) in [p1, p2, p0].into_iter().enumerate() {
        let (chooser, options) = keep_choice(&engine);
        assert_eq!(chooser, player);
        assert_eq!(options.len(), 2);
        assert_eq!(
            kept_announcements(&engine, journal_from),
            [p1, p2, p0][..index]
        );
        keep(&mut engine, player, options[0]);
        if index < 2 {
            assert!(
                rings
                    .iter()
                    .all(|id| engine.state().object(*id).unwrap().zone == Zone::Battlefield),
                "no permanent moves before the last player has chosen"
            );
            pass_to_keep_choice(&mut engine);
        }
    }
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(kept_announcements(&engine, journal_from), vec![p1, p2, p0]);
    for player in [p0, p1, p2] {
        assert_eq!(all_on_battlefield(&engine, player, sol_ring()).len(), 1);
    }
}

/// A completed public choice may not move a card yet, but it changes the next
/// answer's consequences and therefore must live in the replay continuation.
#[test]
fn cataclysm_first_keep_choice_changes_the_replay_fingerprint() {
    let p0 = PlayerId::new(0);
    let setup = || {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[
                    ornithopter(),
                    sol_ring(),
                    index::MEMNITE,
                    index::GRIZZLY_BEARS,
                    plains(),
                    plains(),
                    plains(),
                    plains(),
                ],
            )
            .hand(0, &[cataclysm()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        cast_from_hand(&mut engine, p0, cataclysm());
        pass_to_keep_choice(&mut engine);
        engine
    };
    let mut left = setup();
    let mut right = setup();
    let thopter = on_battlefield(&left, p0, ornithopter()).unwrap();
    let ring = on_battlefield(&left, p0, sol_ring()).unwrap();
    keep(&mut left, p0, thopter);
    keep(&mut right, p0, ring);

    for engine in [&left, &right] {
        let (chooser, options) = keep_choice(engine);
        assert_eq!(chooser, p0);
        assert_eq!(options.len(), 3);
        assert!(options.contains(&thopter));
        assert_eq!(
            all_on_battlefield(engine, p0, index::GRIZZLY_BEARS).len(),
            1
        );
    }
    assert_eq!(left.state().snapshot_hash(), right.state().snapshot_hash());
    assert_ne!(left.snapshot_hash(), right.snapshot_hash());
}

/// Departing while asked does not invent a keep selection or skip the players
/// still at the table. The remaining two choices finish the same resolution.
#[test]
fn cataclysm_current_chooser_leaving_skips_only_that_players_choices() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut engine = three_player_cataclysm();
    let journal_from = engine.state().journal.len();
    assert_eq!(keep_choice(&engine).0, p1);
    engine.apply(p1, PlayerAction::Concede).unwrap();
    assert!(!matches!(engine.pending(), Pending::GameOver(_)));
    assert!(!kept_announcements(&engine, journal_from).contains(&p1));

    pass_to_keep_choice(&mut engine);
    let (chooser, options) = keep_choice(&engine);
    assert_eq!(chooser, p2);
    keep(&mut engine, p2, options[0]);
    pass_to_keep_choice(&mut engine);
    let (chooser, options) = keep_choice(&engine);
    assert_eq!(chooser, p0);
    keep(&mut engine, p0, options[0]);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(kept_announcements(&engine, journal_from), vec![p2, p0]);
    assert!(all_on_battlefield(&engine, p1, sol_ring()).is_empty());
    assert_eq!(all_on_battlefield(&engine, p2, sol_ring()).len(), 1);
    assert_eq!(all_on_battlefield(&engine, p0, sol_ring()).len(), 1);
    assert!(!matches!(engine.pending(), Pending::GameOver(_)));
}
