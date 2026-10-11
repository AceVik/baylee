//! `cards/lands/check/cori_mountain_monastery.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Cori Mountain Monastery is a checkland with the unusual pair — "unless you
/// control a Plains **or** an Island" on a land that taps for {R} — and a
/// condition is untested until both of its branches fire, so it is played
/// twice off one board difference. The Island is the half worth having,
/// because a reader that took only the first of the two printed types would
/// pass the Plains test and fail nothing.
///
#[test]
fn cori_mountain_monastery_checks_for_a_plains_or_an_island_and_taps_for_red() {
    let p0 = PlayerId::new(0);

    // Neither type on the board: the printed default applies.
    let mut bare = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[cori_mountain_monastery()])
        .start();
    keep_mulligans(&mut bare);
    assert!(walk_to_own_main(&mut bare, p0), "p0 reaches its own main");
    let tapped = play_land(&mut bare, p0, cori_mountain_monastery());
    assert!(
        entered_tapped(&bare, tapped),
        "a Forest is neither a Plains nor an Island"
    );

    // An Island, which is the second of the two types the card names.
    let mut checked = Duel::new(SEED, forest())
        .battlefield(0, &[island()])
        .hand(0, &[cori_mountain_monastery()])
        .start();
    keep_mulligans(&mut checked);
    assert!(
        walk_to_own_main(&mut checked, p0),
        "p0 reaches its own main"
    );
    let untapped = play_land(&mut checked, p0, cori_mountain_monastery());
    assert!(
        !entered_tapped(&checked, untapped),
        "\"unless you control a Plains or an Island\" — the Island is the or"
    );

    activate(&mut checked, p0, cori_mountain_monastery(), 0);
    assert_eq!(
        checked.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "{{T}}: Add {{R}}"
    );
    assert_eq!(
        checked.state().players[0].mana_pool.total(),
        1,
        "and nothing else: this land makes one colour"
    );
}

#[test]
fn cori_permission_survives_source_departure_and_the_whole_next_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for on_opponents_turn in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[cori_mountain_monastery()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        if on_opponents_turn {
            reach_their_main_phase(&mut e, p1);
            e.apply(p1, PlayerAction::PassPriority).unwrap();
        }
        e.state.players[0].mana_pool.add(ManaColor::Red, 4);
        e.refresh_offer();
        let card = *e
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .last()
            .unwrap();
        activate(&mut e, p0, cori_mountain_monastery(), 1);
        pass_until(&mut e, stack_is_empty);
        let source = on_battlefield(&e, p0, cori_mountain_monastery()).unwrap();
        bury(&mut e, &[source]);
        if !on_opponents_turn {
            reach_their_main_phase(&mut e, p1);
        }
        assert!(crate::casting::play_permission(e.state(), p0, card).is_some());
        reach_their_main_phase(&mut e, p0);
        assert!(priority_offer(&e).lands.contains(&card));
        reach_their_main_phase(&mut e, p1);
        assert!(crate::casting::play_permission(e.state(), p0, card).is_none());
        assert_eq!(e.state().object(card).unwrap().zone, Zone::Exile);
    }
}

#[test]
fn cori_offers_both_land_and_paid_spell_faces_without_lending_the_permission_to_a_new_object() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for as_land in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[cori_mountain_monastery()])
            .hand(0, &[index::BLACKBLOOM_ROGUE])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let card = hand_to_library_top(&mut e, p0, index::BLACKBLOOM_ROGUE);
        e.state.players[0].mana_pool.add(ManaColor::Red, 4);
        e.refresh_offer();
        activate(&mut e, p0, cori_mountain_monastery(), 1);
        pass_until(&mut e, stack_is_empty);
        assert!(
            !priority_offer(&e).castable.contains(&card),
            "mana cost was not waived"
        );
        e.state.players[0].mana_pool.add(ManaColor::Black, 3);
        e.refresh_offer();
        assert!(priority_offer(&e).castable.contains(&card));
        assert!(priority_offer(&e).lands.contains(&card));
        if as_land {
            e.apply(p0, PlayerAction::PlayLand { card }).unwrap();
            assert_eq!(e.state().object(card).unwrap().face_index, 1);
        } else {
            cast_object_and_resolve(&mut e, p0, card);
        }
        e.state
            .move_object(
                card,
                ZoneLocation::Exile(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
        assert!(crate::casting::play_permission(e.state(), p0, card).is_none());
    }
}
