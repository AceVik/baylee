//! `cards/creatures/mv_5/thrun_breaker_of_silence.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Thrun, Breaker of Silence: the same keyword on a second card, and the
/// trample the first one does not print.
///
/// The counterspell half is the test above, on the other Thrun; what is new
/// here is that the permanent that arrives carries **both** printed
/// keywords. The targeting restriction is
/// `thrun_breaker_of_silence_refuses_only_an_opponents_nongreen_spell`.
#[test]
fn thrun_breaker_of_silence_arrives_with_trample_through_a_counterspell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(383, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[thrun_breaker_of_silence()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let counter = in_hand(&engine, p1, counterspell()).expect("the counterspell is in hand");
    cast_from_hand(&mut engine, p0, thrun_breaker_of_silence());
    let spell = on_stack(&engine, thrun_breaker_of_silence()).expect("the troll is cast");
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    engine
        .apply(p1, PlayerAction::CastSpell { card: counter })
        .expect("the troll is a legal target for the counterspell");
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![spell],
            },
        )
        .expect("the counterspell points at the troll");
    pass_until(&mut engine, stack_is_empty);

    let thrun = on_battlefield(&engine, p0, thrun_breaker_of_silence()).expect("the troll arrived");
    assert!(
        keywords(&engine, thrun).contains(KeywordSet::TRAMPLE),
        "and it tramples, which is the half of the printing that is not the \
         reason it got here"
    );
    assert_eq!(pt(&engine, thrun), (5, 5), "a 5/5 as printed");
}

/// Thrun, Breaker of Silence: "Thrun can't be the target of nongreen spells
/// your opponents control or abilities from nongreen sources your opponents
/// control."
///
/// The opponent's Lightning Bolt is offered the Elves beside Thrun and not
/// Thrun; their Giant Growth is green and is offered Thrun. Thrun's own
/// controller's Swords to Plowshares is white and is offered Thrun too:
/// the sentence names only the opponents' spells.
#[test]
fn thrun_breaker_of_silence_refuses_only_an_opponents_nongreen_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(384, forest())
        .battlefield(0, &[thrun_breaker_of_silence(), llanowar_elves(), plains()])
        .hand(0, &[swords_to_plowshares()])
        .battlefield(1, &[mountain(), forest()])
        .hand(1, &[lightning_bolt(), giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    let thrun = on_battlefield(&engine, p0, thrun_breaker_of_silence()).expect("Thrun is seated");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("so are the Elves");

    reach_their_main_phase(&mut engine, p1);
    let mountain_id = on_battlefield(&engine, p1, mountain()).expect("p1's Mountain");
    tap_mana_where(&mut engine, p1, |id| id == mountain_id);
    cast_with_floating(&mut engine, p1, lightning_bolt());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the Bolt asks for a target, got {:?}", engine.pending())
    };
    assert!(options.contains(&elves), "the Elves may be Bolted");
    assert!(
        !options.contains(&thrun),
        "a red spell an opponent controls can't target Thrun"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: vec![],
            },
        )
        .expect("the Elves were offered");
    pass_until(&mut engine, stack_is_empty);

    cast_from_hand(&mut engine, p1, giant_growth());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Giant Growth asks for a target, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&thrun),
        "a green spell an opponent controls may target Thrun"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![thrun],
                players: vec![],
            },
        )
        .expect("Thrun was offered");
    pass_until(&mut engine, stack_is_empty);

    assert!(walk_to_own_main(&mut engine, p0), "p0's turn comes round");
    cast_from_hand(&mut engine, p0, swords_to_plowshares());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Swords asks for a target, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&thrun),
        "a nongreen spell Thrun's own controller controls may target him"
    );
}
