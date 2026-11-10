//! `cards/lands/utility/seraph_sanctuary.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Seraph Sanctuary prints three lines: "When this land enters, you gain 1
/// life", "Whenever an Angel you control enters, you gain 1 life" and "{T}: Add
/// {C}". All three are played on one board, and each one needs its own witness:
/// the land has to be *played* rather than seated by the harness so its own
/// entry trigger can fire, the mana line is read against a pool that is empty
/// before it (`{T}` is the whole price, so nothing but the tap can have made
/// the mana), and the Angel sentence is fenced in by the two creatures that
/// must not pay — an Elf of mine entering (no Angel, no life) and an Angel
/// entering across the table, which is the only reading that tells
/// `Filter::ControlledByYou` from "an Angel".
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn seraph_sanctuary_gains_a_life_for_itself_and_only_for_angels_of_its_controller() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[seraph_sanctuary(), llanowar_elves(), serra_angel()])
        .battlefield(1, &[plains(), plains(), plains(), plains(), plains()])
        .hand(1, &[serra_angel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "nothing has happened yet"
    );

    // The land arrives the way a land arrives: a real land drop, so its own
    // entry trigger has something to fire off.
    let sanctuary = play_land(&mut engine, p0, seraph_sanctuary());
    assert!(
        !is_tapped(&engine, sanctuary),
        "an ordinary land enters untapped, so its {{T}} is available at once"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        engine.state().players[0].life,
        21,
        "\"When this land enters, you gain 1 life\" — one, and to its controller"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the opponent gains nothing for a land they did not play"
    );

    // The printed "{T}: Add {C}", pressed by hand off a board with an empty
    // pool: the whole price is the land's own tap, so anything in the pool
    // afterwards came off the Sanctuary and nowhere else.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == sanctuary)
        .expect("the Sanctuary's {T}: Add {C} is the only activated ability it prints");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the ability came out of the offer");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    assert!(is_tapped(&engine, sanctuary), "the {{T}} was the price");
    assert!(
        !matches!(engine.pending(), Pending::ChooseColor { .. }),
        "{{C}} is fixed, so the card has nothing to ask on the way: {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "\"{{T}}: Add {{C}}\" — one colourless, off one tap"
    );
    assert_eq!(
        pool.total(),
        1,
        "and it is the first mana on this board, so nothing else was tapped before it"
    );

    // The first control: an Elf of mine entering is not an Angel entering. Every
    // source on the board is tapped here, the Sanctuary included, so the Elf is
    // cast off mana that is really floating.
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the Elf resolved"
    );
    assert_eq!(
        engine.state().players[0].life,
        21,
        "an Elf of mine is no Angel: the second sentence stays quiet"
    );

    // The Angel of mine, which is the sentence under test.
    cast_with_floating(&mut engine, p0, serra_angel());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, serra_angel()).is_some(),
        "the Angel resolved"
    );
    assert_eq!(
        engine.state().players[0].life,
        22,
        "\"Whenever an Angel you control enters, you gain 1 life\" — once, for \
         the one Angel that entered"
    );

    // The second control: an Angel across the table is as much an Angel as
    // mine, so `Filter::ControlledByYou` is the only thing that can decline it.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, serra_angel());
    pass_until(&mut engine, |e| at_rest(e, p1));
    assert!(
        on_battlefield(&engine, p1, serra_angel()).is_some(),
        "their Angel resolved"
    );
    assert_eq!(
        engine.state().players[0].life,
        22,
        "\"an Angel *you* control\": the Angel that just entered is not mine, so \
         my Sanctuary pays me nothing for it"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the Sanctuary belongs to me, not to the seat that cast the Angel"
    );
}
