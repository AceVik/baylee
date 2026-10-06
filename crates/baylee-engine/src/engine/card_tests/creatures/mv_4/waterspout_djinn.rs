//! `cards/creatures/mv_4/waterspout_djinn.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Waterspout Djinn — {2}{U}{U} 4/4 flier: "At the beginning of your upkeep,
/// sacrifice this creature unless you return an untapped Island you control
/// to its owner's hand."
///
/// The Karoo sentence on a creature, and the first card in the pool to write
/// it anywhere but on a land — which is how it arrived: the batch that added
/// it turned `every_land_that_pays_by_returning_one_is_in_the_table` red,
/// because that sweep asks the whole pool and the table it compares against
/// held ten lands. `PAYS_BY_RETURNING_A_LAND_ELSEWHERE` is the row, and this
/// is the test the row promises.
///
/// Both answers, because the two are different rules and a driver that only
/// paid would also pass over a card that never asked: the Island named goes
/// to its owner's hand and the Djinn stays (CR 400.3), and naming nothing is
/// how the player declines, after which the creature is sacrificed. The
/// trigger is an upkeep one, so nothing here plays a card at all — the board
/// is seeded and the question arrives on its own.
#[test]
fn a_djinn_that_costs_a_bounce_pays_it_or_is_sacrificed() {
    let p0 = PlayerId::new(0);
    let djinn = card_index("050dac46-9ba0-4b8a-b61b-1c7ec6f3723a");

    for pay in [true, false] {
        let mut engine = Duel::new(if pay { 940 } else { 941 }, island())
            .battlefield(0, &[djinn, island()])
            .start();
        keep_mulligans(&mut engine);
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });

        let Pending::ChooseCards {
            options,
            prompt,
            min,
            ..
        } = engine.pending().clone()
        else {
            panic!("the upkeep trigger asks what pays: {:?}", engine.pending())
        };
        assert_eq!(
            prompt,
            ChoicePrompt::CostReturn,
            "the price is a bounce, and the client draws the question from the prompt"
        );
        assert_eq!(min, 0, "declining has to be an answer, or paying is forced");
        assert_eq!(
            options.len(),
            1,
            "the one untapped Island is the whole menu"
        );
        let body = on_battlefield(&engine, p0, djinn).expect("the Djinn is on the battlefield");

        let answer = if pay { options.clone() } else { Vec::new() };
        engine
            .apply(p0, PlayerAction::ChooseObjects { objects: answer })
            .unwrap();

        let battlefield = engine.state().zones.list(ZoneLocation::Battlefield);
        if pay {
            assert!(battlefield.contains(&body), "it was paid for and stays");
            assert!(
                engine
                    .state()
                    .zones
                    .list(ZoneLocation::Hand(p0))
                    .contains(&options[0]),
                "what paid goes to its owner's hand (CR 400.3)"
            );
        } else {
            assert!(
                !battlefield.contains(&body),
                "nothing was named, so the sacrifice the card prints happens"
            );
            assert!(
                in_graveyard(&engine, p0, djinn).is_some(),
                "and a sacrificed creature is in its owner's graveyard"
            );
            assert!(
                battlefield.contains(&options[0]),
                "the Island it did not return is untouched"
            );
        }
    }
}
