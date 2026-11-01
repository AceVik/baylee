//! A combat tax's payment window that closes short (CR 508.1i, 509.1e–f):
//! the declaration was never made, what the window did is reversed
//! (CR 732.1), and the same player is asked again.

use super::testkit::*;
use super::*;
use crate::object::Status;
use baylee_core::ids::{CardIndex, Defender};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
const P2: PlayerId = PlayerId::new(2);

/// "Myr Prototype can't attack or block unless you pay {1} for each +1/+1
/// counter on it."
fn myr_prototype() -> CardIndex {
    card_index("496e0fbb-cb01-41d6-9c56-23e3ae2bb4f7")
}

/// "Whenever a permanent becomes untapped, that permanent's controller
/// mills a card."
fn mesmeric_orb() -> CardIndex {
    card_index("03efb4f3-b8e2-4441-824f-886dc40712c4")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Every permanent `seat` controls made from `card`, in battlefield order.
fn all_of(e: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> Vec<ObjectId> {
    e.state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            e.state()
                .object(*id)
                .is_some_and(|o| o.controller == seat && o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}

fn is_tapped(e: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    e.state()
        .object(id)
        .is_some_and(|o| o.status.contains(Status::TAPPED))
}

/// Puts `n` +1/+1 counters on `seat`'s Myr Prototype.
fn counters_on_prototype(e: &mut Engine<RegistryLookup>, seat: PlayerId, n: u16) -> ObjectId {
    let prototype = on_battlefield(e, seat, myr_prototype()).expect("the Prototype");
    e.dev_state_mut(P0)
        .expect("the harness may set boards up")
        .object_mut(prototype)
        .expect("the Prototype")
        .counters
        .set(baylee_cards_dsl::CounterKind::P1P1, n);
    e.refresh_offer();
    prototype
}

/// "No abilities trigger and no effects apply as a result of an undone
/// action" (CR 732.1). Seat 0 declares its Prototype, two counters on it,
/// attacking, taps one of its two Forests in the window and passes short:
/// the attack was never declared, the Forest untaps as the window is
/// reversed, and Mesmeric Orb, which hears every permanent becoming
/// untapped, does not hear that one. Nothing is milled.
#[test]
fn an_attack_tax_window_given_back_untaps_without_a_trigger() {
    let mut e = Duel::new(7_321, basic_forest())
        .battlefield(
            0,
            &[
                mesmeric_orb(),
                myr_prototype(),
                basic_forest(),
                basic_forest(),
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let prototype = counters_on_prototype(&mut e, P0, 2);
    let forest = all_of(&e, P0, basic_forest())[0];
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let library = e.state().zones.list(ZoneLocation::Library(P0)).len();
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(prototype, Defender::Player(P1))],
        },
    )
    .unwrap();
    assert!(
        matches!(e.pending(), Pending::Priority { player: P0, .. }),
        "the attack tax opens a payment window: {:?}",
        e.pending()
    );
    tap_mana_where(&mut e, P0, |id| id == forest);
    assert!(is_tapped(&e, forest), "one Forest is tapped in the window");
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    assert!(
        !e.state().combat.is_attacking(prototype),
        "{{1}} short of {{2}}"
    );
    assert!(!is_tapped(&e, forest), "the window is given back");
    assert!(
        matches!(e.pending(), Pending::ChooseAttackers { player: P0, .. }),
        "seat 0 declares again: {:?}",
        e.pending()
    );
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: Vec::new(),
        },
    )
    .unwrap();
    let now = e.state().turn.number;
    pass_until(&mut e, |e| e.state().turn.number > now);
    assert!(
        e.state().zones.list(ZoneLocation::Graveyard(P0)).is_empty(),
        "the reversed untap milled nobody"
    );
    assert_eq!(
        e.state().zones.list(ZoneLocation::Library(P0)).len(),
        library
    );
}

/// "The first defending player declares all their blocks, then the second
/// defending player, and so on" (CR 802.4). Seat 0 attacks seats 1 and 2;
/// seat 1 declares no blocks, and seat 2 declares its Prototype blocking and
/// leaves the window short. Seat 1's declaration stands: it is seat 2 who is
/// asked again, not the first defending player.
#[test]
fn a_block_tax_declined_asks_the_player_who_declined_it() {
    let mut e = Duel::table(7_322, basic_forest(), 3)
        .battlefield(0, &[grizzly_bears(), grizzly_bears()])
        .battlefield(2, &[myr_prototype(), basic_forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let prototype = counters_on_prototype(&mut e, P2, 1);
    let bears = all_of(&e, P0, grizzly_bears());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![
                (bears[0], Defender::Player(P1)),
                (bears[1], Defender::Player(P2)),
            ],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        matches!(e.pending(), Pending::ChooseBlockers { player: P1, .. }),
        "the first defending player first: {:?}",
        e.pending()
    );
    e.apply(
        P1,
        PlayerAction::DeclareBlockers {
            blockers: Vec::new(),
        },
    )
    .unwrap();
    let Pending::ChooseBlockers {
        player: P2,
        blockers,
        ..
    } = e.pending().clone()
    else {
        panic!("then the second: {:?}", e.pending());
    };
    assert!(
        blockers.iter().any(|o| o.blocker == prototype),
        "seat 2's Forest reaches the {{1}}: {blockers:?}"
    );
    e.apply(
        P2,
        PlayerAction::DeclareBlockers {
            blockers: vec![(prototype, bears[1])],
        },
    )
    .unwrap();
    assert!(
        matches!(e.pending(), Pending::Priority { player: P2, .. }),
        "the block tax opens a payment window: {:?}",
        e.pending()
    );
    e.apply(P2, PlayerAction::PassPriority).unwrap();
    assert!(!e.state().combat.is_blocking(prototype, bears[1]));
    assert!(
        matches!(e.pending(), Pending::ChooseBlockers { player: P2, .. }),
        "the player who declined declares again, and nobody before them: {:?}",
        e.pending()
    );
    e.apply(
        P2,
        PlayerAction::DeclareBlockers {
            blockers: Vec::new(),
        },
    )
    .expect("an empty declaration ends the declarations");
}
