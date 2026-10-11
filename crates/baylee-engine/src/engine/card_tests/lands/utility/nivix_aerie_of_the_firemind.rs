//! `cards/lands/utility/nivix_aerie_of_the_firemind.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mana production does not exile a card or consume the other ability.
#[test]
fn nivix_offers_both_abilities_and_its_mana_activation_only_makes_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), mountain(), mountain()])
        .hand(0, &[nivix_aerie_of_the_firemind()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, nivix_aerie_of_the_firemind());
    assert!(
        on_battlefield(&engine, p0, nivix_aerie_of_the_firemind()).is_some(),
        "the land drop put it onto the battlefield"
    );
    assert!(
        !is_tapped(&engine, land),
        "a land enters untapped unless it prints otherwise, so the tap below \
         is the card's own and not the entry"
    );

    let library_before = library_size(&engine, p0);
    let exiled_before = engine.state().zones.list(ZoneLocation::Exile(p0)).len();

    // Mana first: `{2}{U}{R}` is in the pool, so an ability the offer does
    // not carry was not declined for want of mana.
    tap_all_mana_but(&mut engine, p0, Some(nivix_aerie_of_the_firemind()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "two Islands and two Mountains are exactly {{U}}{{U}}{{R}}{{R}}"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    let offered: Vec<(ObjectId, u32)> = legal
        .abilities
        .iter()
        .copied()
        .filter(|(source, _)| *source == land)
        .collect();
    assert_eq!(
        offered,
        vec![(land, 0), (land, 1)],
        "both printed abilities are affordable"
    );

    // The half that is written.
    activate(&mut engine, p0, nivix_aerie_of_the_firemind(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "the {{C}} the card prints, off the card's own {{T}}"
    );
    assert_eq!(pool.total(), 5, "and the four basics made nothing extra");
    assert!(is_tapped(&engine, land), "which tapped it");

    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "a mana activation does not exile anything"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(p0)).len(),
        exiled_before,
        "and the exile zone is the empty one the game started with"
    );
}

#[test]
fn nivix_permissions_use_the_spell_face_and_expire_as_your_next_turn_begins() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for (definition, allowed) in [
        (index::DARK_RITUAL, true),
        (index::TWINING_TWINS, true),
        (index::ORNITHOPTER, false),
        (index::BLACKBLOOM_ROGUE, false),
        (index::FOREST, false),
    ] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[nivix_aerie_of_the_firemind(), index::GRIZZLY_BEARS])
            .hand(0, &[definition])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let card = hand_to_library_top(&mut e, p0, definition);
        for color in [
            ManaColor::Red,
            ManaColor::Blue,
            ManaColor::Black,
            ManaColor::White,
        ] {
            e.state.players[0].mana_pool.add(color, 10);
        }
        e.refresh_offer();
        activate(&mut e, p0, nivix_aerie_of_the_firemind(), 1);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().object(card).unwrap().zone, Zone::Exile);
        let offer = priority_offer(&e);
        assert_eq!(offer.castable.contains(&card), allowed, "{definition:?}");
        assert!(!offer.lands.contains(&card));
        if definition == index::TWINING_TWINS {
            e.apply(p0, PlayerAction::CastSpell { card }).unwrap();
            // The creature front must never appear alongside its legal instant Adventure.
            let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
            aim_at(&mut e, p0, bear);
            assert_eq!(e.state().object(card).unwrap().face_index, 1);
            continue;
        }
        reach_their_main_phase(&mut e, p1);
        assert!(crate::casting::play_permission(e.state(), p0, card).is_some());
        reach_their_main_phase(&mut e, p0);
        assert!(crate::casting::play_permission(e.state(), p0, card).is_none());
    }
}

#[test]
fn exile_play_windows_follow_actual_next_turns_including_extra_and_skipped_turns() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for source in [nivix_aerie_of_the_firemind(), cori_mountain_monastery()] {
        for skipped in [false, true] {
            let mut e = Duel::new(SEED, forest())
                .battlefield(0, &[source, index::TIME_VAULT])
                .start();
            keep_mulligans(&mut e);
            reach_main_phase(&mut e, p0);
            for color in [ManaColor::Red, ManaColor::Blue] {
                e.state.players[0].mana_pool.add(color, 4);
            }
            e.refresh_offer();
            let card = *e
                .state()
                .zones
                .list(ZoneLocation::Library(p0))
                .last()
                .unwrap();
            activate(&mut e, p0, source, 1);
            pass_until(&mut e, stack_is_empty);
            e.state.extra_turns.push_front(p0);
            let vault = on_battlefield(&e, p0, index::TIME_VAULT).unwrap();
            e.state.set_tapped(vault, false);
            if skipped {
                e.state.set_tapped(vault, true);
                pass_until(&mut e, |e| {
                    matches!(
                        e.pending(),
                        Pending::YesNo {
                            prompt: YesNoPrompt::SkipTurn { .. },
                            ..
                        }
                    )
                });
                e.apply(p0, PlayerAction::YesNo(true)).unwrap();
                reach_their_main_phase(&mut e, p1);
                assert!(
                    crate::casting::play_permission(e.state(), p0, card).is_some(),
                    "a skipped turn is not the next turn"
                );
            }
            pass_until(&mut e, |e| {
                e.state().players[0].turns_started == 2 && e.state().turn.phase == Phase::FirstMain
            });
            assert_eq!(
                crate::casting::play_permission(e.state(), p0, card).is_some(),
                source == cori_mountain_monastery()
            );
            reach_their_main_phase(&mut e, p1);
            assert!(crate::casting::play_permission(e.state(), p0, card).is_none());
        }
    }
}
