//! `cards/sorceries/mv_3/cleansing.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::YesNoPrompt;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cleansing() -> CardIndex {
    card_index("46b83657-1699-4162-a4bb-b792da396fe6")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Seat 0 holds Cleansing with {W}{W}{W} floating; the boards are as given
/// and Cleansing is cast, ready to ask who pays.
fn cast(mine: &[CardIndex], theirs: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .hand(0, &[cleansing()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    lk::float(&mut engine, P0, &[(ManaColor::White, 3)]);
    cast_with_floating(&mut engine, P0, cleansing());
    engine
}

/// Answers every "pay 1 life?" question with `answer(position, asked)` until
/// the spell is done; returns who was asked, in order.
#[track_caller]
fn answer_all(
    engine: &mut Engine<RegistryLookup>,
    mut answer: impl FnMut(usize, PlayerId) -> bool,
) -> Vec<PlayerId> {
    let mut asked = Vec::new();
    for _ in 0..40 {
        pass_until(engine, |e| {
            stack_is_empty(e) || matches!(e.pending(), Pending::YesNo { .. })
        });
        let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
            return asked;
        };
        assert!(
            matches!(prompt, YesNoPrompt::PayLife { amount: 1 }),
            "the question is whether to pay 1 life, got {prompt:?}"
        );
        let yes = answer(asked.len(), player);
        asked.push(player);
        engine
            .apply(player, PlayerAction::YesNo(yes))
            .expect("the question takes a yes or a no");
    }
    panic!("Cleansing never stopped asking");
}

/// Cleansing (46b83657-1699-4162-a4bb-b792da396fe6) — "For each land,
/// destroy that land unless any player pays 1 life." Nobody pays: every land
/// of both players is destroyed, each land asking each player in turn order,
/// and nothing else is touched.
#[test]
fn cleansing_destroys_every_land_when_nobody_pays() {
    let mut engine = cast(
        &[forest(), mountain(), grizzly_bears()],
        &[plains(), island(), grizzly_bears()],
    );
    let asked = answer_all(&mut engine, |_, _| false);

    assert_eq!(
        asked,
        vec![P0, P1, P0, P1, P0, P1, P0, P1],
        "four lands, two players each"
    );
    for (seat, land) in [
        (P0, forest()),
        (P0, mountain()),
        (P1, plains()),
        (P1, island()),
    ] {
        assert!(
            on_battlefield(&engine, seat, land).is_none(),
            "the land is destroyed"
        );
        assert!(
            in_graveyard(&engine, seat, land).is_some(),
            "into its owner's graveyard"
        );
    }
    assert!(
        on_battlefield(&engine, P0, grizzly_bears()).is_some(),
        "a creature is no land"
    );
    assert!(on_battlefield(&engine, P1, grizzly_bears()).is_some());
    assert_eq!(lk::life(&engine, P0), 20);
    assert_eq!(lk::life(&engine, P1), 20);
}

/// Cleansing — "unless any player pays 1 life": the opponent pays for every
/// land, from their own life, and every land survives. A land that has been
/// paid for asks nobody else.
#[test]
fn cleansing_lands_survive_when_any_player_pays() {
    let mut engine = cast(&[forest(), mountain()], &[plains(), island()]);
    let asked = answer_all(&mut engine, |_, who| who == P1);

    assert_eq!(
        asked,
        vec![P0, P1, P0, P1, P0, P1, P0, P1],
        "you decline, they pay, per land"
    );
    for (seat, land) in [
        (P0, forest()),
        (P0, mountain()),
        (P1, plains()),
        (P1, island()),
    ] {
        assert!(
            on_battlefield(&engine, seat, land).is_some(),
            "paid for, so it stays"
        );
    }
    assert_eq!(lk::life(&engine, P0), 20, "I paid nothing");
    assert_eq!(lk::life(&engine, P1), 16, "1 life for each of four lands");
}

/// Cleansing — one payment saves one land: I pay for my Forest (19 life),
/// nobody pays for the opponent's Plains, which is destroyed.
#[test]
fn cleansing_a_payment_saves_only_its_own_land() {
    let mut engine = cast(&[forest()], &[plains()]);
    let asked = answer_all(&mut engine, |position, _| position == 0);

    assert_eq!(
        asked,
        vec![P0, P0, P1],
        "the Forest asks me and is paid for; the Plains asks both"
    );
    assert!(on_battlefield(&engine, P0, forest()).is_some());
    assert!(on_battlefield(&engine, P1, plains()).is_none());
    assert_eq!(lk::life(&engine, P0), 19);
    assert_eq!(lk::life(&engine, P1), 20);
}
