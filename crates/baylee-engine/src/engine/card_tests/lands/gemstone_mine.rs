//! `cards/lands/gemstone_mine.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gemstone Mine, which is the same card asked one question: "Add one mana
/// of any color."
///
/// That question is the reason it is a separate test. A colour choice
/// **suspends** the resolution (CR 605.3b resolves a mana ability without
/// the stack, but a choice still parks it in `Engine::resolution`), so the
/// sacrifice clause is on the far side of an answer the player has not
/// given yet.
///
/// The shape is not new — every pain land has it, `{T}: Add {W} or {U}.
/// This land deals 1 damage to you.` being a colour question with an effect
/// behind it — but no engine test had ever played one. So this is the first
/// test of the resume-then-trailing-effect path, and what it covers is
/// wider than the one card.
///
/// So the middle of the last activation is asserted, and it is the whole
/// point of the test: the cost has been paid (no counters left) and the
/// land is **still on the battlefield**, because the effect that kills it
/// has not run. Answer the colour and both halves land together.
#[test]
fn gemstone_mine_dies_on_the_far_side_of_the_colour_it_asks_for() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(553, forest()).hand(0, &[gemstone_mine()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, gemstone_mine());
    assert!(
        !entered_tapped(&engine, land),
        "Gemstone Mine prints no `tapped`, so it works the turn it arrives"
    );
    assert_eq!(
        counters_on(&engine, land, counters::MINING),
        3,
        "three mining counters, which is three activations"
    );

    for turn in 0..3u16 {
        if turn > 0 {
            cross_into_the_next_own_main(&mut engine, p0);
        }
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: land,
                    ability_index: 0,
                },
            )
            .expect("a mining counter is there to pay with");

        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!(
                "turn {turn}: any colour is a choice: {:?}",
                engine.pending()
            )
        };
        assert_eq!(options.len(), 5, "turn {turn}: all five colours");
        assert_eq!(
            counters_on(&engine, land, counters::MINING),
            2 - turn,
            "turn {turn}: the cost is paid before the question is asked"
        );
        assert!(
            in_graveyard(&engine, p0, gemstone_mine()).is_none(),
            "turn {turn}: and the clause that would sacrifice it has not run \
             yet, because the resolution is parked on this question"
        );

        engine
            .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
            .expect("a colour the engine offered");
        assert_eq!(
            engine.state().players[0]
                .mana_pool
                .available(ManaColor::Green),
            1,
            "turn {turn}: one green, from a land that prints no green symbol"
        );
    }

    assert!(
        in_graveyard(&engine, p0, gemstone_mine()).is_some(),
        "the third answer emptied the land and the clause after the mana \
         sacrificed it"
    );
}
