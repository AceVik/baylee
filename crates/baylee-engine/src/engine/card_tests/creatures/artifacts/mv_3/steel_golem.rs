//! `cards/creatures/artifacts/mv_3/steel_golem.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn steel_golem() -> CardIndex {
    card_index("b5cd9c55-34ad-4e1f-92b2-527c9babc4d4")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn nature_s_resurgence() -> CardIndex {
    card_index("276f49ee-7cf3-4a4d-9b14-74aedcbef69f")
}

/// Whether the card may be cast now, mana already made or still to be made.
fn may_cast(e: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> bool {
    let id = in_hand(e, seat, card).expect("the card is in hand");
    let Pending::Priority { legal, .. } = e.pending() else {
        return false;
    };
    legal.castable.contains(&id) || legal.payable.contains(&id)
}

/// Steel Golem (b5cd9c55-34ad-4e1f-92b2-527c9babc4d4) — "You can't cast
/// creature spells." Its controller may cast the Bears neither with the
/// Forests untapped nor with the mana floating, but a noncreature spell
/// ({2}{G}{G} Nature's Resurgence) is still castable with the same mana.
#[test]
fn steel_golem_stops_its_controller_casting_creature_spells_only() {
    let mut e = lk::game(
        SEED,
        &[steel_golem(), forest(), forest(), forest(), forest()],
        &[],
        &[grizzly_bears(), nature_s_resurgence()],
        &[],
    );
    assert!(
        !may_cast(&e, P0, grizzly_bears()),
        "no creature spells, even payable ones"
    );
    assert!(
        may_cast(&e, P0, nature_s_resurgence()),
        "other spells are fine"
    );

    tap_all_mana(&mut e, P0);
    assert!(
        !lk::castable(&e, P0, grizzly_bears()),
        "not with the mana floating either"
    );
    assert!(lk::castable(&e, P0, nature_s_resurgence()));
}

/// Steel Golem — "**You** can't cast": the opponent casts creature spells as
/// usual. On player 1's own turn their Bears is castable.
#[test]
fn steel_golem_does_not_stop_the_opponent_casting_creature_spells() {
    let mut e = lk::game(
        SEED,
        &[steel_golem()],
        &[forest(), forest()],
        &[],
        &[grizzly_bears()],
    );
    pass_until(&mut e, |e| {
        e.state().turn.active == P1
            && e.state().turn.step == Step::Main
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P1)
    });
    assert!(
        may_cast(&e, P1, grizzly_bears()),
        "the Golem's controller is the only one bound"
    );
}
