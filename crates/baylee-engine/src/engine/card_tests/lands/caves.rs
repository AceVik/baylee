//! The tests of the cards under `cards/lands/caves/`, one module per card.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

mod captivating_cave;
mod cavernous_maw;
mod echoing_deeps;
mod forgotten_monument;
mod hidden_cataract;
mod hidden_courtyard;
mod hidden_necropolis;
mod hidden_nursery;
mod hidden_volcano;
mod pit_of_offerings;
mod secret_tunnel;
mod sunken_citadel;
mod sunken_palace;
mod urza_s_cave;
mod volatile_fault;

/// Exercise each colored discover activation through its paid stack ability.
fn hidden_cave_discovers(card: CardIndex, color: ManaColor, cast: bool) {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[card])
        .hand(
            0,
            &[index::GRIZZLY_BEARS, index::SERRA_ANGEL, index::MOUNTAIN],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let cave = on_battlefield(&e, p0, card).unwrap();
    let bear = hand_to_library_top(&mut e, p0, index::GRIZZLY_BEARS);
    let angel = hand_to_library_top(&mut e, p0, index::SERRA_ANGEL);
    let mountain = hand_to_library_top(&mut e, p0, index::MOUNTAIN);
    let state = e.dev_state_mut(p0).unwrap();
    state.set_tapped(cave, false);
    state.players[0].mana_pool.add(color, 5);
    e.refresh_offer();
    activate(&mut e, p0, card, 1);
    assert_eq!(e.state().object(cave).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::Discover { .. },
                ..
            }
        )
    });
    assert!(
        matches!(e.pending(), Pending::YesNo { prompt: crate::choice::YesNoPrompt::Discover { card }, .. } if *card==bear)
    );
    let library = e.state().zones.list(ZoneLocation::Library(p0));
    assert!(library[..2].contains(&angel) && library[..2].contains(&mountain));
    e.apply(p0, PlayerAction::YesNo(cast)).unwrap();
    if cast {
        assert_eq!(e.state().object(bear).unwrap().zone, Zone::Stack);
    }
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(bear).unwrap().zone,
        if cast { Zone::Battlefield } else { Zone::Hand }
    );
}
