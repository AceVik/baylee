#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch46_beacon_requires_different_colors_and_pays_only_planeswalkers() {
    let mut e = prepared(
        &[index::AMINATOU_THE_FATESHIFTER, index::SWORDS_TO_PLOWSHARES],
        &[index::INTERPLANAR_BEACON],
        &[index::GRIZZLY_BEARS],
    );
    let beacon = source(&e, index::INTERPLANAR_BEACON);
    pool(&mut e, &[(ManaColor::Green, 1)]);
    ability(&mut e, beacon, 2);
    assert_eq!(colors(&e).len(), 5);
    color(&mut e, ManaColor::Blue);
    assert_eq!(
        colors(&e),
        vec![
            ManaColor::White,
            ManaColor::Black,
            ManaColor::Red,
            ManaColor::Green
        ]
    );
    let unchanged = e.snapshot_hash();
    assert!(
        e.apply(P0, PlayerAction::ChooseColor(ManaColor::Blue))
            .is_err()
    );
    assert_eq!(e.snapshot_hash(), unchanged);
    color(&mut e, ManaColor::White);
    assert_eq!(e.state.players[0].mana_pool.total(), 2);
    let swords = in_hand(&e, P0, index::SWORDS_TO_PLOWSHARES).unwrap();
    no_mana_to_cast(&e, swords);
    e.state.players[0].mana_pool.add(ManaColor::Black, 1);
    e.refresh_offer();
    let walker = in_hand(&e, P0, index::AMINATOU_THE_FATESHIFTER).unwrap();
    let life = e.state.players[0].life;
    e.apply(P0, PlayerAction::CastSpell { card: walker })
        .unwrap();
    assert_eq!(e.state.players[0].mana_pool.total(), 0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(walker).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state.players[0].life, life + 1);
}

#[test]
fn batch46_beacon_ignores_loyalty_activations_and_opponents_planeswalker_casts() {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(1, &[index::AMINATOU_THE_FATESHIFTER])
        .battlefield(
            0,
            &[
                index::INTERPLANAR_BEACON,
                index::AMINATOU_THE_FATESHIFTER,
                index::GRIZZLY_BEARS,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let life = e.state.players[0].life;
    let walker = on_battlefield(&e, P0, index::AMINATOU_THE_FATESHIFTER).unwrap();
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    ability(&mut e, walker, 1);
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![bear],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[0].life, life);
    pass_until(&mut e, |e| {
        e.state.turn.active == P1
            && matches!(e.state.turn.phase, crate::turn::Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player: P1, .. })
    });
    for color in [ManaColor::White, ManaColor::Blue, ManaColor::Black] {
        e.state.players[1].mana_pool.add(color, 1);
    }
    e.refresh_offer();
    let spell = in_hand(&e, P1, index::AMINATOU_THE_FATESHIFTER).unwrap();
    e.apply(P1, PlayerAction::CastSpell { card: spell })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[0].life, life);
    assert!(on_battlefield(&e, P1, index::AMINATOU_THE_FATESHIFTER).is_some());
}
