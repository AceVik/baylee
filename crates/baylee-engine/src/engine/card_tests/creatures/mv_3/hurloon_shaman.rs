//! `cards/creatures/mv_3/hurloon_shaman.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::mana::ManaColor;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn hurloon_shaman() -> CardIndex {
    card_index("9fc74fc9-bb02-4d9e-bb8e-f44f219d8946")
}

fn lightning_bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}

/// Hurloon Shaman (9fc74fc9-bb02-4d9e-bb8e-f44f219d8946) — "When this creature
/// dies, each player sacrifices a land of their choice." A Bolt kills it. Each
/// player is asked once, about their own lands only (the Shaman itself and
/// the other player's lands are not on the menu), and the land they do not
/// pick stays: P0 keeps one Mountain, P1 keeps the Plains and loses the
/// Island they chose.
#[test]
fn hurloon_shaman_makes_each_player_sacrifice_a_land_of_their_choice() {
    let mut e = lk::game(
        SEED,
        &[hurloon_shaman(), mountain(), mountain()],
        &[plains(), island()],
        &[lightning_bolt()],
        &[],
    );
    let shaman = lk::obj(&e, P0, hurloon_shaman());
    let island_id = lk::obj(&e, P1, island());

    lk::float(&mut e, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut e, P0, lightning_bolt());
    aim_at(&mut e, P0, shaman);

    let mut asked = Vec::new();
    for _ in 0..2 {
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        let Pending::ChooseCards {
            player, options, ..
        } = e.pending().clone()
        else {
            unreachable!("the predicate just matched")
        };
        assert_eq!(options.len(), 2, "two lands each: {options:?}");
        for id in &options {
            let object = e.state().object(*id).expect("object exists");
            assert_eq!(object.controller, player, "only their own lands");
        }
        asked.push(player);
        let pick = if player == P1 { island_id } else { options[0] };
        assert!(options.contains(&pick));
        e.apply(
            player,
            PlayerAction::ChooseObjects {
                objects: vec![pick],
            },
        )
        .unwrap();
    }
    pass_until(&mut e, stack_is_empty);

    assert!(
        asked.contains(&P0) && asked.contains(&P1),
        "each player was asked: {asked:?}"
    );
    assert!(in_graveyard(&e, P0, hurloon_shaman()).is_some());
    assert!(
        in_graveyard(&e, P0, mountain()).is_some(),
        "P0 gave up a Mountain"
    );
    assert!(
        on_battlefield(&e, P0, mountain()).is_some(),
        "and kept the other"
    );
    assert!(
        in_graveyard(&e, P1, island()).is_some(),
        "P1 chose the Island"
    );
    assert!(
        on_battlefield(&e, P1, plains()).is_some(),
        "so the Plains stays"
    );
}
