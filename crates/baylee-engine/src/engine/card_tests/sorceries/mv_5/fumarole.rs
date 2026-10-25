//! `cards/sorceries/mv_5/fumarole.rs`, played against all printed clauses.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn fumarole() -> CardIndex {
    card_index("a9e33138-e1c0-4b69-b868-1ce8493c073d")
}

fn prepared(
    mine: &[CardIndex],
    theirs: &[CardIndex],
    responses: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut e = lk::game(SEED, mine, theirs, &[fumarole()], responses);
    lk::float(
        &mut e,
        P0,
        &[
            (ManaColor::Colorless, 3),
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
        ],
    );
    e
}

fn announce(e: &mut Engine<RegistryLookup>, creature: ObjectId, land: ObjectId) {
    cast_with_floating(e, P0, fumarole());
    lk::aim(e, &[creature], &[]);
    assert_eq!(lk::life(e, P0), 20, "targets precede life payment");
    let Pending::ChooseTargets {
        options, min, max, ..
    } = e.pending()
    else {
        panic!("target land")
    };
    assert!(options.contains(&land));
    assert!(!options.contains(&creature));
    assert_eq!((*min, *max), (1, 1));
    lk::aim(e, &[land], &[]);
    assert_eq!(lk::life(e, P0), 17, "the casting cost is already paid");
}

/// Both targets are mandatory and unrestricted by controller. Both are
/// destroyed, and the three life is paid during casting.
#[test]
fn fumarole_pays_life_and_destroys_a_creature_and_a_land() {
    for own_creature in [false, true] {
        let (mine, theirs) = if own_creature {
            (vec![index::GRIZZLY_BEARS], vec![index::FOREST])
        } else {
            (vec![index::FOREST], vec![index::GRIZZLY_BEARS])
        };
        let mut e = prepared(&mine, &theirs, &[]);
        let creature = lk::obj(&e, if own_creature { P0 } else { P1 }, index::GRIZZLY_BEARS);
        let land = lk::obj(&e, if own_creature { P1 } else { P0 }, index::FOREST);
        announce(&mut e, creature, land);
        lk::settle(&mut e);
        assert_eq!(lk::zone_of(&e, creature), Zone::Graveyard);
        assert_eq!(lk::zone_of(&e, land), Zone::Graveyard);
        assert_eq!(lk::life(&e, P0), 17);
    }
}

/// Fumarole requires one creature and one land before it can be offered.
#[test]
fn fumarole_is_not_offered_without_both_target_kinds() {
    for board in [vec![index::GRIZZLY_BEARS], vec![index::FOREST]] {
        let mut e = prepared(&[], &board, &[]);
        let spell = in_hand(&e, P0, fumarole()).unwrap();
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority")
        };
        assert!(!legal.castable.contains(&spell) && !legal.payable.contains(&spell));
        assert!(
            e.apply(P0, PlayerAction::CastSpell { card: spell })
                .is_err()
        );
        assert_eq!(lk::life(&e, P0), 20);
    }
}

/// The surviving target still resolves if the creature returns to its
/// owner's hand. A real Counterspell instead stops both destructions, but
/// the life payment remains spent.
#[test]
fn fumarole_keeps_life_paid_when_countered_or_one_target_leaves() {
    for countered in [false, true] {
        let response = if countered {
            index::COUNTERSPELL
        } else {
            index::UNSUMMON
        };
        let mut e = prepared(&[], &[index::GRIZZLY_BEARS, index::FOREST], &[response]);
        let creature = lk::obj(&e, P1, index::GRIZZLY_BEARS);
        let land = lk::obj(&e, P1, index::FOREST);
        announce(&mut e, creature, land);
        let spell = on_stack(&e, fumarole()).unwrap();
        pass_until(
            &mut e,
            |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
        );
        lk::float(&mut e, P1, &[(ManaColor::Blue, 2)]);
        cast_with_floating(&mut e, P1, response);
        lk::aim(&mut e, &[if countered { spell } else { creature }], &[]);
        lk::settle(&mut e);
        assert_eq!(lk::life(&e, P0), 17);
        assert_eq!(
            lk::zone_of(&e, land),
            if countered {
                Zone::Battlefield
            } else {
                Zone::Graveyard
            }
        );
        assert_eq!(
            lk::zone_of(&e, creature),
            if countered {
                Zone::Battlefield
            } else {
                Zone::Hand
            }
        );
    }
}

/// Both targets are destroyed simultaneously. The dying Vampire therefore
/// sees Dryad Arbor (a land creature) die and its one trigger still resolves.
#[test]
fn fumarole_destroys_both_target_groups_in_one_simultaneous_event() {
    let mut e = prepared(&[index::VINDICTIVE_VAMPIRE, index::DRYAD_ARBOR], &[], &[]);
    let vampire = lk::obj(&e, P0, index::VINDICTIVE_VAMPIRE);
    let arbor = lk::obj(&e, P0, index::DRYAD_ARBOR);
    announce(&mut e, vampire, arbor);
    lk::settle(&mut e);
    assert_eq!(lk::zone_of(&e, vampire), Zone::Graveyard);
    assert_eq!(lk::zone_of(&e, arbor), Zone::Graveyard);
    assert_eq!(
        lk::life(&e, P0),
        18,
        "pay three, then gain one from Arbor's death"
    );
    assert_eq!(
        lk::life(&e, P1),
        19,
        "the Vampire observed the simultaneous death"
    );
}
