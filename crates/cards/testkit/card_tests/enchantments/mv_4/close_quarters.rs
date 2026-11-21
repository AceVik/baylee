//! `cards/enchantments/mv_4/close_quarters.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn close_quarters() -> CardIndex {
    card_index("c3f841c4-6e12-46fc-802a-3025182e7ff4")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn objs(e: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> Vec<ObjectId> {
    e.state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            e.state()
                .object(id)
                .is_some_and(|o| o.controller == seat && o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}

/// Whenever a creature you control becomes blocked (once even if double-blocked;
/// not for unblocked attackers or opponent's attackers), deals 1 damage to any
/// target.
#[test]
fn close_quarters_triggers_once_per_blocked_attacker_you_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[
            close_quarters(),
            grizzly_bears(),
            savannah_lions(),
            savannah_lions(),
        ],
        &[llanowar_elves(), llanowar_elves(), grizzly_bears()],
        &[],
        &[],
    );
    let my_bear = lk::obj(&e, p0, grizzly_bears());
    let my_lions = objs(&e, p0, savannah_lions());
    let opp_elves = objs(&e, p1, llanowar_elves());
    let opp_bear = lk::obj(&e, p1, grizzly_bears());

    // Attack with Grizzly Bears + 1 Savannah Lions; P1 double-blocks Grizzly Bears -> 1 trigger.
    lk::attack(&mut e, &[my_bear, my_lions[0]]);
    lk::block(&mut e, &[(opp_elves[0], my_bear), (opp_elves[1], my_bear)]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    lk::aim(&mut e, &[opp_elves[0]], &[]);
    lk::through_combat(&mut e);
    lk::settle(&mut e);

    assert_eq!(lk::zone_of(&e, opp_elves[0]), Zone::Graveyard);
    assert_eq!(lk::life(&e, p1), 18);

    // Opponent's attacker blocked by untapped my_lions[1] does not trigger P0's Close Quarters.
    reach_their_main_phase(&mut e, p1);
    lk::attack(&mut e, &[opp_bear]);
    lk::block(&mut e, &[(my_lions[1], opp_bear)]);
    pass_until(&mut e, |e| {
        assert!(!matches!(e.pending(), Pending::ChooseTargets { .. }));
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
}
