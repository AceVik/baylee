//! `cards/creatures/mv_3/hooded_blightfang.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn hooded_blightfang() -> CardIndex {
    card_index("342eb666-838c-43d0-bf52-2f704f2a80e3")
}

/// Each of two Snakes triggers for each deathtouch attacker: two Snakes
/// attacking beside an ordinary Ogre produce four drains, not one per source.
/// All combat damage to the player leaves the opposing planeswalker untouched.
#[test]
fn hooded_blightfang_drains_once_per_deathtouch_attacker() {
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[hooded_blightfang(), hooded_blightfang(), gray_ogre()])
        .battlefield(1, &[karn_the_great_creator()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let snakes: Vec<ObjectId> = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            e.state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == hooded_blightfang()))
        })
        .collect();
    assert_eq!(snakes.len(), 2);
    assert!(
        snakes
            .iter()
            .all(|id| { keywords(&e, *id).contains(KeywordSet::DEATHTOUCH) })
    );
    let ogre = on_battlefield(&e, P0, gray_ogre()).unwrap();
    assert!(!keywords(&e, ogre).contains(KeywordSet::DEATHTOUCH));
    let karn = on_battlefield(&e, P1, karn_the_great_creator()).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: snakes
                .iter()
                .chain(std::iter::once(&ogre))
                .map(|id| (*id, Defender::Player(P1)))
                .collect(),
        },
    )
    .unwrap();
    pass_until(&mut e, |e| at_rest(e, P0));
    assert_eq!(e.state().players[0].life, 24, "four independent life gains");
    assert_eq!(
        e.state().players[1].life,
        16,
        "four independent life losses"
    );

    pass_until(&mut e, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(e.state().players[0].life, 24);
    assert_eq!(
        e.state().players[1].life,
        12,
        "two power one plus the Ogre's two"
    );
    assert_eq!(counters_on(&e, karn, CounterKind::Loyalty), 5);
}

#[track_caller]
fn hooded_stomp(
    source_card: CardIndex,
    target_card: CardIndex,
) -> (Engine<RegistryLookup>, ObjectId) {
    let mut p0_board = vec![hooded_blightfang(), forest(), forest()];
    if source_card != hooded_blightfang() {
        p0_board.push(source_card);
    }
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &p0_board)
        .battlefield(1, &[karn_the_great_creator(), llanowar_elves()])
        .hand(0, &[stump_stomp()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let source = on_battlefield(&e, P0, source_card).unwrap();
    let target = on_battlefield(&e, P1, target_card).unwrap();
    let karn = on_battlefield(&e, P1, karn_the_great_creator()).unwrap();
    tap_all_mana(&mut e, P0);
    cast_front_face(&mut e, P0, stump_stomp());
    aim_at(&mut e, P0, source);
    aim_at(&mut e, P0, target);
    pass_until(&mut e, |e| at_rest(e, P0));
    (e, karn)
}

/// Noncombat damage qualifies too: Stump Stomp makes the one-power Snake deal
/// one to Karn, which survives the damage with four loyalty before Hooded
/// Blightfang's triggered ability destroys it.
#[test]
fn hooded_blightfang_destroys_a_planeswalker_damaged_by_your_deathtouch_creature() {
    let (e, karn) = hooded_stomp(hooded_blightfang(), karn_the_great_creator());
    assert_eq!(e.state().object(karn).unwrap().zone, Zone::Graveyard);
}

/// The source filter is not merely "a creature you control": the same damage
/// from an ordinary Ogre removes loyalty but does not destroy the planeswalker.
#[test]
fn hooded_blightfang_does_not_destroy_a_planeswalker_damaged_by_an_ordinary_creature() {
    let (e, karn) = hooded_stomp(gray_ogre(), karn_the_great_creator());
    assert_eq!(e.state().object(karn).unwrap().zone, Zone::Battlefield);
    assert_eq!(counters_on(&e, karn, CounterKind::Loyalty), 3);
}

/// Damage to another kind of permanent does not satisfy the recipient filter.
#[test]
fn hooded_blightfang_does_not_destroy_a_planeswalker_when_a_creature_is_damaged() {
    let (e, karn) = hooded_stomp(hooded_blightfang(), llanowar_elves());
    assert!(
        in_graveyard(&e, P1, llanowar_elves()).is_some(),
        "deathtouch kills the Elf"
    );
    assert_eq!(e.state().object(karn).unwrap().zone, Zone::Battlefield);
    assert_eq!(counters_on(&e, karn, CounterKind::Loyalty), 5);
}
