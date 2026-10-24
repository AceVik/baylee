//! `cards/creatures/mv_6/goblin_marshal.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::color::Color;
use baylee_core::generated::subtypes;

fn goblin_marshal() -> CardIndex {
    card_index("4bc91d7d-37aa-475c-8fe9-763975d51add")
}

fn goblin_token_count(e: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    e.state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|&&id| {
            e.state().object(id).is_some_and(|o| {
                o.controller == seat
                    && o.card.is_none()
                    && o.characteristics().power == Some(1)
                    && o.characteristics().toughness == Some(1)
                    && o.characteristics().colors.contains(Color::Red)
                    && o.characteristics()
                        .subtypes
                        .contains(subtypes::creature::GOBLIN)
            })
        })
        .count()
}

/// Goblin Marshal (4bc91d7d-37aa-475c-8fe9-763975d51add) — "Echo {4}{R}{R} /
/// When this creature enters or dies, create two 1/1 red Goblin creature
/// tokens."
#[test]
fn goblin_marshal_creates_two_goblins_on_etb_and_two_more_when_sacrificed_to_echo() {
    let mut e = lk::game(SEED, &[], &[], &[goblin_marshal()], &[]);
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 6)]);
    cast_with_floating(&mut e, lk::P0, goblin_marshal());
    lk::settle(&mut e);

    assert!(on_battlefield(&e, lk::P0, goblin_marshal()).is_some());
    assert_eq!(
        goblin_token_count(&e, lk::P0),
        2,
        "two 1/1 red Goblins on ETB"
    );

    reach_their_main_phase(&mut e, lk::P1);
    pass_until(&mut e, |e| {
        e.state().turn.active == lk::P0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { .. })
    });
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 6)]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, false);
    lk::settle(&mut e);

    assert!(in_graveyard(&e, lk::P0, goblin_marshal()).is_some());
    assert_eq!(
        goblin_token_count(&e, lk::P0),
        4,
        "dying to Echo creates two more 1/1 red Goblins"
    );
}

/// Goblin Marshal — paying Echo keeps it on the battlefield, and bouncing it to
/// hand is not dying so no extra Goblin tokens are created.
#[test]
fn goblin_marshal_paying_echo_keeps_it_and_bouncing_does_not_trigger_dies() {
    let mut e = lk::game(SEED, &[], &[], &[goblin_marshal(), unsummon()], &[]);
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 6)]);
    cast_with_floating(&mut e, lk::P0, goblin_marshal());
    lk::settle(&mut e);
    assert_eq!(goblin_token_count(&e, lk::P0), 2);

    reach_their_main_phase(&mut e, lk::P1);
    pass_until(&mut e, |e| {
        e.state().turn.active == lk::P0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { .. })
    });
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 6)]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, true);
    reach_main_phase(&mut e, lk::P0);

    let marshal = lk::obj(&e, lk::P0, goblin_marshal());
    assert_eq!(goblin_token_count(&e, lk::P0), 2);

    lk::float(&mut e, lk::P0, &[(ManaColor::Blue, 1)]);
    lk::cast_at(&mut e, lk::P0, unsummon(), &[marshal], &[]);
    assert!(in_hand(&e, lk::P0, goblin_marshal()).is_some());
    assert_eq!(
        goblin_token_count(&e, lk::P0),
        2,
        "returning to hand is not dying"
    );
}
