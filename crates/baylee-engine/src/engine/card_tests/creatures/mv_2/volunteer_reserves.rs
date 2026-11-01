//! `cards/creatures/mv_2/volunteer_reserves.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_cards_dsl::counters;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn volunteer_reserves() -> CardIndex {
    card_index("a2dabd52-0b75-42d7-a07d-0df7b38646e2")
}

/// The tax question of P0's upkeep on `turn`, and the generic mana it names.
fn upkeep_price(e: &mut Engine<RegistryLookup>, turn: u32) -> u16 {
    pass_until(e, |e| {
        e.state().turn.number == turn && matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = e.pending().clone()
    else {
        panic!("the upkeep asks to pay mana: {:?}", e.pending())
    };
    assert_eq!(player, P0);
    mana
}

/// Volunteer Reserves (a2dabd52-0b75-42d7-a07d-0df7b38646e2) — "Banding" and
/// "Cumulative upkeep {1}". It is a 2/4 with banding. The first upkeep puts an
/// age counter and asks {1}, paid with a Plains; the opponent's upkeep adds
/// nothing; the next of its controller's asks {2} (two counters, {1} each),
/// declined, it is sacrificed.
#[test]
fn volunteer_reserves_asks_one_more_each_of_its_controllers_upkeeps() {
    let mut e = Duel::new(SEED, plains())
        .battlefield(0, &[volunteer_reserves(), plains(), plains()])
        .start();
    keep_mulligans(&mut e);
    let reserves = on_battlefield(&e, P0, volunteer_reserves()).expect("the Reserves are out");
    assert_eq!(pt(&e, reserves), (2, 4));
    assert!(
        keywords(&e, reserves).contains(KeywordSet::BANDING),
        "it has banding"
    );

    assert_eq!(upkeep_price(&mut e, 1), 1, "one age counter, {{1}}");
    assert_eq!(counters_on(&e, reserves, counters::AGE), 1);
    let plains_id = lk::obj(&e, P0, plains());
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    e.apply(
        P0,
        PlayerAction::PayWindow {
            steps: vec![PlayerAction::ActivateManaAbility { source: plains_id }],
        },
    )
    .expect("a Plains pays {1}");
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, volunteer_reserves()).is_some(),
        "paid, so it stays"
    );

    reach_their_main_phase(&mut e, P1);
    assert_eq!(
        counters_on(&e, reserves, counters::AGE),
        1,
        "P1's upkeep is not yours"
    );

    assert_eq!(upkeep_price(&mut e, 3), 2, "two age counters, {{1}} each");
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(
        in_graveyard(&e, P0, volunteer_reserves()).is_some(),
        "unpaid: sacrificed"
    );
}
