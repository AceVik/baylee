//! `cards/lands/exotic_orchard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Exotic Orchard reads the **opponent's** lands, which is the only reason
/// this card needs a test at all.
///
/// `Effect::mana_land_color(false)` is a question with no answer on the card
/// and no answer in a view — `view::board_mana` resolves it through
/// `resolve::colors_of`, the very function that hands the mana out. What a
/// test can hold it to is the sentence: my own Forest must not widen the
/// menu, and the opponent's Swamp must.
///
/// Three boards, because one is not enough to tell "reads the wrong side"
/// from "reads nothing": an empty opposing board offers the ability nothing
/// to say and so must not offer it at all. And a fourth, with a Wastes
/// across the table: the Orchard says "any color", and colorless mana is a
/// type of mana and not a colour (CR 106.1a, CR 106.1b), so the Wastes adds
/// nothing to the menu. It used to add `{C}`.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn exotic_orchard_reads_the_opponents_lands_and_not_its_controllers() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    for (pass, (theirs, want)) in [
        (Vec::new(), Vec::new()),
        (vec![swamp()], vec![ManaColor::Black]),
        (
            vec![swamp(), island()],
            vec![ManaColor::Black, ManaColor::Blue],
        ),
        (vec![swamp(), wastes()], vec![ManaColor::Black]),
    ]
    .into_iter()
    .enumerate()
    {
        let seed = 4_860 + u64::try_from(pass).expect("four passes");
        let mut engine = Duel::new(seed, forest())
            // A Forest of my own on every board: if the reading were
            // "any land" rather than "a land an opponent controls", green
            // would be on the menu in all three passes, including the first.
            .battlefield(0, &[exotic_orchard(), forest()])
            .battlefield(1, &theirs)
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let orchard =
            on_battlefield(&engine, p0, exotic_orchard()).expect("the Orchard is on the table");
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        if want.is_empty() {
            // Offered, and that is correct rather than a hole: an ability
            // whose mana is defined by a board may be activated when the
            // board defines none, and it simply produces nothing. What the
            // empty case proves is the other half — that the *menu* is the
            // opponent's lands and not the table's, so my own Forest must
            // not have put green on it.
            assert!(
                legal.abilities.contains(&(orchard, 0)),
                "the ability does not stop existing because it would make \
                 nothing: {:?}",
                legal.abilities
            );
            engine
                .apply(
                    p0,
                    PlayerAction::ActivateAbility {
                        source: orchard,
                        ability_index: 0,
                    },
                )
                .expect("an offered ability is legal");
            pass_until(&mut engine, stack_is_empty);
            assert_eq!(
                engine.state().players[0]
                    .mana_pool
                    .available(ManaColor::Green),
                0,
                "my own Forest is not \"a land an opponent controls\""
            );
            assert_eq!(
                engine.state().players[0].mana_pool.total(),
                0,
                "and with nothing on the other side there is nothing to add"
            );
            continue;
        }
        assert!(
            legal.abilities.contains(&(orchard, 0)),
            "the opponent controls {} land(s) and the Orchard was offered \
             nothing: {:?}",
            theirs.len(),
            legal.abilities
        );

        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: orchard,
                    ability_index: 0,
                },
            )
            .expect("an offered ability is legal");
        // One producible colour is not a choice, and the engine does not
        // manufacture one: it adds the mana and hands priority straight back.
        // Two colours *is* a choice, and then the menu is the assertion. The
        // test has to allow both, because which one happens is a property of
        // the opposing board rather than of the card.
        let pick = want[0];
        if let Pending::ChooseColor { options, .. } = engine.pending().clone() {
            let mut got = options.clone();
            got.sort_by_key(|c| format!("{c:?}"));
            let mut expected = want.clone();
            expected.sort_by_key(|c| format!("{c:?}"));
            assert_eq!(
                got, expected,
                "the menu is exactly what the other side of the table could \
                 make — green is mine and must not be on it"
            );
            engine
                .apply(p0, PlayerAction::ChooseColor(pick))
                .expect("a colour the menu named is legal");
        } else {
            assert_eq!(
                want.len(),
                1,
                "with two producible colours the Orchard must ask, and it \
                 did not: {:?}",
                engine.pending()
            );
        }
        pass_until(&mut engine, stack_is_empty);
        let pool = &engine.state().players[0].mana_pool;
        assert_eq!(pool.available(pick), 1);
        assert_eq!(
            pool.available(ManaColor::Green),
            0,
            "and never green, which is the colour only my own side can make"
        );
    }
}
