#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn tecutlan_discover_captures_chosen_x_and_survives_countered_triggering_spell() {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(0, &[index::WALKING_BALLISTA, index::HILL_GIANT])
        .hand(1, &[index::COUNTERSPELL])
        .battlefield(0, &[index::BRASS_S_TUNNEL_GRINDER])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let rift = source(&e, P0, index::BRASS_S_TUNNEL_GRINDER);
    e.state.switch_face(
        rift,
        RegistryLookup.card(index::BRASS_S_TUNNEL_GRINDER).unwrap(),
        1,
    );
    e.sync_static_effects();
    e.state.players[0].mana_pool = baylee_core::mana::ManaPool::new();
    let found = in_hand(&e, P0, index::HILL_GIANT).unwrap();
    e.state
        .move_object(
            found,
            ZoneLocation::Library(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    ability(&mut e, P0, index::BRASS_S_TUNNEL_GRINDER, 0);
    e.state.players[0].mana_pool.add(ManaColor::Colorless, 5);
    e.refresh_offer();
    let ballista = in_hand(&e, P0, index::WALKING_BALLISTA).unwrap();
    e.apply(P0, PlayerAction::CastSpell { card: ballista })
        .unwrap();
    e.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    assert_eq!(
        e.state.players[0].mana_pool.total(),
        0,
        "actual R from Rift paid generic portion of X X"
    );
    priority(&mut e, P1);
    e.state.players[1].mana_pool.add(ManaColor::Blue, 2);
    e.refresh_offer();
    let counter = in_hand(&e, P1, index::COUNTERSPELL).unwrap();
    e.apply(P1, PlayerAction::CastSpell { card: counter })
        .unwrap();
    targets(&mut e, vec![ballista], vec![]);
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::Discover { .. },
                ..
            }
        )
    });
    assert_eq!(e.state.object(ballista).unwrap().zone, Zone::Graveyard);
    let Pending::YesNo {
        prompt: YesNoPrompt::Discover { card },
        ..
    } = e.pending()
    else {
        unreachable!()
    };
    assert_eq!(
        *card, found,
        "X3 means mana value6, not printed zero: MV4 Giant qualifies"
    );
    yes(&mut e, false);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(found).unwrap().zone, Zone::Hand);
}

#[test]
fn brass_leng_replaced_discard_draws_selected_count_but_does_not_descend() {
    let mut e = prepared(
        &[index::BRASS_S_TUNNEL_GRINDER, index::FOREST],
        &[index::LIBRARY_OF_LENG],
        &[],
    );
    let land = in_hand(&e, P0, index::FOREST).unwrap();
    cast(&mut e, index::BRASS_S_TUNNEL_GRINDER);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    choose(&mut e, vec![land]);
    let before = e.state.zones.list(ZoneLocation::Hand(P0)).len();
    assert!(matches!(
        e.pending(),
        Pending::Arrange {
            prompt: crate::choice::ArrangePrompt::DiscardToLibrary,
            ..
        }
    ));
    e.apply(
        P0,
        PlayerAction::Arrange {
            piles: vec![vec![], vec![land]],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.zones.list(ZoneLocation::Hand(P0)).len(),
        before + 1,
        "discard one (replaced to library), draw two"
    );
    let grinder = source(&e, P0, index::BRASS_S_TUNNEL_GRINDER);
    pass_until(&mut e, |e| {
        e.state.turn.step == crate::turn::Step::End
            && stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { .. })
    });
    assert_eq!(
        e.state
            .object(grinder)
            .unwrap()
            .counters
            .get(CounterKind::Bore),
        0,
        "land never reached graveyard"
    );
}
