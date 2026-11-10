//! `cards/sorceries/mv_1/metamorphosis.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Metamorphosis — {G} sorcery: "As an additional cost to cast this spell,
/// sacrifice a creature. Add X mana of any one color, where X is 1 plus the
/// sacrificed creature's mana value. Spend this mana only to cast creature
/// spells."
///
/// The sacrificed creature is Canopy Spider, mana value two, so X is three —
/// one more than the creature's own mana value, the sentence's arithmetic
/// and not the printed `{1}{G}` (CR 202.3 for the value being read off the
/// permanent as it last existed, CR 608.2h).
///
/// The question is exactly one creature: every creature p0 controls is on
/// its menu, and naming two of them is refused, which is the official ruling
/// ("You must sacrifice exactly one creature to cast this spell; you cannot
/// cast it without sacrificing a creature, and you cannot sacrifice
/// additional creatures").
///
/// The mana is one entry of one chosen colour and no other — "any one
/// color" is one pick for the whole amount — and it is filed as restricted
/// mana rather than in the plain counters, which is the engine's form of
/// "Spend this mana only to cast creature spells".
#[test]
#[allow(clippy::too_many_lines)] // one cast, the cost paid, the colour chosen, the pool read
fn metamorphosis_sacrifices_exactly_one_creature_for_its_mana_value_plus_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), canopy_spider(), llanowar_elves()])
        .hand(0, &[metamorphosis()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let spider = on_battlefield(&engine, p0, canopy_spider()).expect("the sacrifice");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the creature left behind");

    // The Forest and only the Forest pays {G}: the Elf is a legal sacrifice
    // and must not be spent as a mana source before the cost asks for it.
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest");
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: land })
        .expect("a Forest pays {G}");
    engine
        .apply(
            p0,
            PlayerAction::CastSpell {
                card: in_hand(&engine, p0, metamorphosis()).expect("the sorcery is in hand"),
            },
        )
        .expect("one Forest pays {G}");

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the additional cost is asked at cast, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0);
    assert_eq!(
        (min, max),
        (1, 1),
        "\"sacrifice a creature\" is exactly one"
    );
    assert!(
        options.contains(&spider) && options.contains(&elf),
        "both of p0's creatures may pay it: {options:?}"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![spider, elf],
                },
            )
            .is_err(),
        "you cannot sacrifice additional creatures"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![spider],
            },
        )
        .expect("exactly one creature is a legal payment");

    // The mana arrives as the spell resolves, so the one colour is chosen
    // then, and the choice covers the whole amount.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        unreachable!("the walk stops on exactly this")
    };
    assert_eq!(player, p0, "the caster picks the colour");
    assert_eq!(options.len(), 5, "\"any one color\" is every colour");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black is one of the five");
    pass_until(&mut engine, stack_is_empty);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.total(),
        3,
        "1 plus Canopy Spider's mana value of 2, and no other mana anywhere"
    );
    assert_eq!(
        pool.available(ManaColor::Black),
        0,
        "the mana is restricted to creature spells (\"Spend this mana only to \
         cast creature spells\") and plain counters hold none of it"
    );
    assert_eq!(
        pool.restricted().len(),
        1,
        "one pick for the whole amount: not three entries and not five colours"
    );
    let entry = pool.restricted()[0];
    assert_eq!(
        (entry.color, entry.amount),
        (ManaColor::Black, 3),
        "three black mana, exactly {{X}} where X is 1 + 2"
    );
    for color in ManaColor::ALL {
        assert_eq!(
            pool.available(color),
            0,
            "no plain counter of {color:?} was touched"
        );
    }

    assert!(
        in_graveyard(&engine, p0, canopy_spider()).is_some(),
        "the sacrificed creature is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the creature that did not pay is still on the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, metamorphosis()).is_some(),
        "the resolved sorcery went to its owner's graveyard"
    );
    assert!(
        is_tapped(&engine, land),
        "the Forest's {{G}} paid the spell"
    );
}

/// The other half of the additional cost: with no creature on the board the
/// cost cannot be paid, so the spell cannot be cast. The mana is floating
/// when the offer is read, so "not castable" is the missing creature and not
/// an unaffordable `{G}`.
#[test]
fn metamorphosis_is_not_offered_without_a_creature_to_sacrifice() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[metamorphosis()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = on_battlefield(&engine, p0, forest()).expect("the Forest");
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: land })
        .expect("a Forest pays {G}");
    let spell = in_hand(&engine, p0, metamorphosis()).expect("the sorcery is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&spell),
        "with {{G}} floating and no creature to sacrifice the additional cost \
         has no legal payment, so the spell is not offered: {:?}",
        legal.castable
    );
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: spell })
            .is_err(),
        "naming it anyway is refused"
    );
}
