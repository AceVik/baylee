//! `cards/lands/cycling/glittering_massif.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Glittering Massif is a Mountain Plains that enters tapped and cycles for
/// `{2}`. One board reads all three printed lines: a copy that has stood since
/// before the turn is the control that separates "this land is tapped because
/// of its own sentence" from "everything here is tapped", the played copy is
/// the entry replacement itself, and the second copy in hand is the one that
/// cycles. The two colours are the whole of the mana line — a Mountain Plains
/// makes `{R}` *or* `{W}`, so the engine has to ask rather than pick.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn glittering_massif_enters_tapped_taps_for_red_or_white_and_cycles_for_two() {
    let p0 = PlayerId::new(0);
    // `starting_battlefield` places a permanent with `Cause::Setup`, so no
    // replacement effect ever looks at it: this Massif is untapped *because*
    // it was seeded, which is exactly what makes it the contrast for the one
    // played below rather than a second copy of the same claim.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), glittering_massif()])
        .hand(0, &[glittering_massif(), glittering_massif()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ready = on_battlefield(&engine, p0, glittering_massif()).expect("a Massif is out");
    assert!(!is_tapped(&engine, ready), "and it is untapped");

    // Both lists: a land taps for mana by virtue of its basic land types,
    // which is the CR 305.6 shortcut and an entry with no index to name
    // (#159), while a printed `{T}: Add …` arrives in `abilities`.
    let taps_now = |engine: &Engine<RegistryLookup>, land: ObjectId| -> bool {
        let Pending::Priority { legal, .. } = engine.pending() else {
            return false;
        };
        legal.mana_abilities.contains(&land)
            || legal.abilities.iter().any(|(source, _)| *source == land)
    };
    assert!(
        taps_now(&engine, ready),
        "an untapped Massif offers its {{T}}"
    );

    let played = play_land(&mut engine, p0, glittering_massif());
    assert_ne!(played, ready, "the land drop made a second Massif");
    assert!(is_tapped(&engine, played), "\"This land enters tapped.\"");
    assert!(
        !taps_now(&engine, played),
        "and a land that entered tapped has no {{T}} to offer this turn"
    );
    assert!(
        taps_now(&engine, ready),
        "while the Massif that was already out is untouched by the newcomer's rule"
    );

    // Two Forests pay the cycling's {2}. The Massifs are named as the thing
    // kept back — the played one is tapped anyway, but the ready one is the
    // permanent this test reads at the end, and `tap_all_mana` would have
    // spent it here (rule 17).
    tap_all_mana_but(&mut engine, p0, Some(glittering_massif()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests, two green, and no Massif among them"
    );

    let cycler = in_hand(&engine, p0, glittering_massif()).expect("a second Massif is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(source, _)| *source == cycler)
        .expect("cycling is an activated ability of the card in hand");
    assert_eq!(
        ability_index, 1,
        "the mana line is ability 0 and the cycling line is ability 1"
    );
    let library = library_size(&engine, p0);
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("two green is the {{2}} the line asks for");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, glittering_massif()).is_some(),
        "cycling discards this very card to pay its own cost"
    );
    assert!(
        in_hand(&engine, p0, glittering_massif()).is_none(),
        "so the other copy has left the hand"
    );
    assert_eq!(
        library_size(&engine, p0),
        library - 1,
        "\"Discard this card: Draw a card.\" — one off the top"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand,
        "the discarded card and the drawn card are one card each"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{2}} came out of the pool"
    );

    // The mana line, off the Massif that never entered this turn. The pool is
    // empty, so the one mana it makes is the whole of what is in it (CR 500.5
    // has already emptied the cycling's green).
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let tap = if legal.mana_abilities.contains(&ready) {
        PlayerAction::ActivateManaAbility { source: ready }
    } else {
        let (_, index) = legal
            .abilities
            .iter()
            .copied()
            .find(|(source, _)| *source == ready)
            .expect("a Mountain Plains taps for mana on one list or the other");
        PlayerAction::ActivateAbility {
            source: ready,
            ability_index: index,
        }
    };
    engine.apply(p0, tap).expect("the land is untapped");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{R}} or {{W}}` on a land that has both types is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped is the one that names it");
    assert_eq!(options.len(), 2, "the Mountain and the Plains: {options:?}");
    assert!(
        options.contains(&ManaColor::White) && options.contains(&ManaColor::Red),
        "and neither of them is missing: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("white was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "the other half of the choice is not simply given away with it"
    );
    assert_eq!(pool.total(), 1, "one land, one mana");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, ready), "and it paid its own {{T}}");
}
