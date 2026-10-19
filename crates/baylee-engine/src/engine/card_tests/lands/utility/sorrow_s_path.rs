//! `cards/lands/utility/sorrow_s_path.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn sorrow_s_path() -> CardIndex {
    card_index("5419eae0-3c12-4748-9cf8-d927752e527e")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

/// Every copy of `card` `seat` controls on the battlefield, in play order.
fn all_of(engine: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == seat && o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}

/// Sorrow's Path (5419eae0-3c12-4748-9cf8-d927752e527e) — "{T}: Choose two
/// target blocking creatures controlled by the same opponent. If each of
/// those creatures could block all creatures that the other is blocking,
/// remove both of them from combat. Each one then blocks all creatures the
/// other was blocking." Two Hill Giants attack, Grizzly Bears blocks one and
/// Savannah Lions the other; after the ability the blocks are crossed.
#[test]
fn sorrow_s_path_swaps_what_two_blockers_block() {
    let mut engine = lk::game(
        SEED,
        &[sorrow_s_path(), hill_giant(), hill_giant()],
        &[grizzly_bears(), savannah_lions()],
        &[],
        &[],
    );
    let giants = all_of(&engine, P0, hill_giant());
    assert_eq!(giants.len(), 2, "two Hill Giants attack");
    let bears = lk::obj(&engine, P1, grizzly_bears());
    let lions = lk::obj(&engine, P1, savannah_lions());

    let offer = lk::attack(&mut engine, &giants);
    for blocker in [bears, lions] {
        for giant in &giants {
            assert!(
                lk::may_block(&offer, blocker, *giant),
                "either blocker may block either Giant"
            );
        }
    }
    lk::block(&mut engine, &[(bears, giants[0]), (lions, giants[1])]);
    lk::to_step(&mut engine, P0, Step::DeclareBlockers);
    assert_eq!(engine.state().combat.blocked_by(bears), vec![giants[0]]);
    assert_eq!(engine.state().combat.blocked_by(lions), vec![giants[1]]);

    activate(&mut engine, P0, sorrow_s_path(), 0);
    lk::aim(&mut engine, &[bears, lions], &[]);
    lk::settle(&mut engine);

    assert_eq!(
        engine.state().combat.blocked_by(bears),
        vec![giants[1]],
        "the Bears now block what the Lions blocked"
    );
    assert_eq!(
        engine.state().combat.blocked_by(lions),
        vec![giants[0]],
        "the Lions now block what the Bears blocked"
    );
    assert_eq!(engine.state().combat.blockers_of(giants[0]), vec![lions]);
    assert_eq!(engine.state().combat.blockers_of(giants[1]), vec![bears]);
}

/// Sorrow's Path — "Whenever this land becomes tapped, it deals 2 damage to
/// you and each creature you control." Tapping it for its ability costs its
/// controller 2 life and puts 2 damage on each of their creatures, and none
/// on the opponent's.
#[test]
fn sorrow_s_path_hurts_its_controller_and_their_creatures_when_tapped() {
    let mut engine = lk::game(
        SEED,
        &[sorrow_s_path(), hill_giant(), hill_giant()],
        &[grizzly_bears(), savannah_lions()],
        &[],
        &[],
    );
    let giants = all_of(&engine, P0, hill_giant());
    let bears = lk::obj(&engine, P1, grizzly_bears());
    let lions = lk::obj(&engine, P1, savannah_lions());
    lk::attack(&mut engine, &giants);
    lk::block(&mut engine, &[(bears, giants[0]), (lions, giants[1])]);
    lk::to_step(&mut engine, P0, Step::DeclareBlockers);
    assert_eq!(lk::life(&engine, P0), 20);

    activate(&mut engine, P0, sorrow_s_path(), 0);
    lk::aim(&mut engine, &[bears, lions], &[]);
    lk::settle(&mut engine);

    assert_eq!(lk::life(&engine, P0), 18, "2 damage to its controller");
    assert_eq!(lk::life(&engine, P1), 20, "the opponent takes none");
    for giant in &giants {
        assert_eq!(
            lk::damage_on(&engine, *giant),
            2,
            "2 damage on each of mine"
        );
        assert_eq!(lk::zone_of(&engine, *giant), Zone::Battlefield);
    }
    assert_eq!(lk::damage_on(&engine, bears), 0, "not on the opponent's");
    assert_eq!(lk::damage_on(&engine, lions), 0, "not on the opponent's");
    assert!(
        lk::tapped(&engine, lk::obj(&engine, P0, sorrow_s_path())),
        "the land is tapped"
    );
}

/// Sorrow's Path — "If each of those creatures could block all creatures
/// that the other is blocking …". A Grizzly Bears cannot block a Serra Angel
/// (flying), so the Angel and the Bears do not trade places: the blocks stay
/// as they were (the tap and its damage still happened).
#[test]
fn sorrow_s_path_does_nothing_when_one_blocker_could_not_block_the_others_attacker() {
    let mut engine = lk::game(
        SEED,
        &[sorrow_s_path(), serra_angel(), hill_giant()],
        &[serra_angel(), grizzly_bears()],
        &[],
        &[],
    );
    let my_angel = lk::obj(&engine, P0, serra_angel());
    let giant = lk::obj(&engine, P0, hill_giant());
    let their_angel = lk::obj(&engine, P1, serra_angel());
    let bears = lk::obj(&engine, P1, grizzly_bears());

    let offer = lk::attack(&mut engine, &[my_angel, giant]);
    assert!(
        !lk::may_block(&offer, bears, my_angel),
        "Grizzly Bears cannot block a flyer"
    );
    assert!(lk::may_block(&offer, their_angel, giant));
    lk::block(&mut engine, &[(their_angel, my_angel), (bears, giant)]);
    lk::to_step(&mut engine, P0, Step::DeclareBlockers);

    activate(&mut engine, P0, sorrow_s_path(), 0);
    lk::aim(&mut engine, &[their_angel, bears], &[]);
    lk::settle(&mut engine);

    assert_eq!(
        engine.state().combat.blocked_by(their_angel),
        vec![my_angel],
        "the Angel still blocks the Angel"
    );
    assert_eq!(
        engine.state().combat.blocked_by(bears),
        vec![giant],
        "the Bears still block the Giant"
    );
    assert_eq!(
        lk::life(&engine, P0),
        18,
        "the land was tapped all the same"
    );
}
