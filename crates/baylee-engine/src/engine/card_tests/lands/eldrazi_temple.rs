//! `cards/lands/eldrazi_temple.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn eldrazi_temple_adds_restricted_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(105, forest())
        .battlefield(0, &[eldrazi_temple()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let temple = on_battlefield(&engine, p0, eldrazi_temple()).expect("Eldrazi Temple deployed");
    activate(&mut engine, p0, eldrazi_temple(), 1);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 0);
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 2);
    assert_eq!(pool.restricted()[0].color, ManaColor::Colorless);
    assert!(is_tapped(&engine, temple));
}

/// "Spend this mana only to cast colorless Eldrazi spells": no Maskwood Nexus
/// anywhere, so what makes a spell an Eldrazi is its own type line. Drowner of
/// Truth (Devoid, Eldrazi) is the one colorless Eldrazi spell; the others each
/// miss exactly one word — a colorless artifact that is no Eldrazi, a colored
/// Eldrazi (a changeling is every creature type, Eldrazi too), a green non-Eldrazi.
#[test]
fn eldrazi_temple_double_colorless_pays_only_for_colorless_eldrazi_spells() {
    use crate::casting::{SpendFor, pay_mana_for, spendable_units};
    use baylee_core::generated::index;
    use baylee_core::mana::ManaCost;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::ELDRAZI_TEMPLE])
        .hand(
            0,
            &[
                index::DROWNER_OF_TRUTH,
                index::MIND_STONE,
                index::CRIB_SWAP,
                index::GRIZZLY_BEARS,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    activate(&mut e, p0, index::ELDRAZI_TEMPLE, 1);
    assert_eq!(e.state().players[0].mana_pool.total(), 2, "{{C}}{{C}}");
    let drowner = in_hand(&e, p0, index::DROWNER_OF_TRUTH).unwrap();
    assert_eq!(
        spendable_units(e.state(), p0, SpendFor::Spell(drowner)),
        2,
        "a colorless Eldrazi spell (Devoid) may be paid with it"
    );
    for (card, why) in [
        (
            index::MIND_STONE,
            "a colorless artifact spell that is not an Eldrazi",
        ),
        (
            index::CRIB_SWAP,
            "a white Eldrazi (changeling) spell is not colorless",
        ),
        (index::GRIZZLY_BEARS, "a green non-Eldrazi creature spell"),
    ] {
        let spell = in_hand(&e, p0, card).unwrap();
        assert_eq!(
            spendable_units(e.state(), p0, SpendFor::Spell(spell)),
            0,
            "refused: {why}"
        );
        let before = e.state().players[0].mana_pool.clone();
        assert!(
            pay_mana_for(
                e.dev_state_mut(p0).unwrap(),
                p0,
                SpendFor::Spell(spell),
                &ManaCost::parse("{1}")
            )
            .is_none(),
            "no payment of {{1}} with this mana: {why}"
        );
        assert_eq!(
            e.state().players[0].mana_pool,
            before,
            "the refused payment is atomic"
        );
    }
    assert!(
        pay_mana_for(
            e.dev_state_mut(p0).unwrap(),
            p0,
            SpendFor::Spell(drowner),
            &ManaCost::parse("{1}")
        )
        .is_some(),
        "control: the same payment for the colorless Eldrazi spell goes through"
    );
}

/// "…or activate abilities of colorless Eldrazi": a colorless Eldrazi
/// permanent (Drowner of Truth, Devoid) may be paid for; a colorless
/// non-Eldrazi (Walking Ballista, Mind Stone) and a green non-Eldrazi may not.
#[test]
fn eldrazi_temple_double_colorless_pays_only_for_abilities_of_colorless_eldrazi() {
    use crate::casting::{SpendFor, spendable_units};
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                index::ELDRAZI_TEMPLE,
                index::DROWNER_OF_TRUTH,
                index::ORNITHOPTER,
                index::MIND_STONE,
                index::LLANOWAR_ELVES,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    activate(&mut e, p0, index::ELDRAZI_TEMPLE, 1);
    assert_eq!(e.state().players[0].mana_pool.total(), 2, "{{C}}{{C}}");
    let drowner = on_battlefield(&e, p0, index::DROWNER_OF_TRUTH).unwrap();
    assert_eq!(
        spendable_units(e.state(), p0, SpendFor::Ability(drowner)),
        2,
        "an ability of a colorless Eldrazi may be paid with it"
    );
    for (card, why) in [
        (index::ORNITHOPTER, "a colorless non-Eldrazi creature"),
        (index::MIND_STONE, "a colorless non-Eldrazi artifact"),
        (index::LLANOWAR_ELVES, "a green non-Eldrazi creature"),
    ] {
        let source = on_battlefield(&e, p0, card).unwrap();
        assert_eq!(
            spendable_units(e.state(), p0, SpendFor::Ability(source)),
            0,
            "refused: an ability of {why}"
        );
    }
}

/// Colored Eldrazi *permanents* need a Maskwood Nexus in this pool (no printed
/// colored Eldrazi creature exists), so the colored half of "colorless
/// Eldrazi" is shown here, with the Nexus making every creature every type:
/// the colorless Ornithopter may be paid for, the green Elves — an Eldrazi now —
/// may not.
#[test]
fn eldrazi_temple_double_colorless_refuses_a_colored_eldrazi_even_under_maskwood_nexus() {
    use crate::casting::{SpendFor, spendable_units};
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                index::ELDRAZI_TEMPLE,
                index::MASKWOOD_NEXUS,
                index::ORNITHOPTER,
                index::LLANOWAR_ELVES,
            ],
        )
        .hand(0, &[index::ORNITHOPTER, index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    activate(&mut e, p0, index::ELDRAZI_TEMPLE, 1);
    let thopter_on_board = on_battlefield(&e, p0, index::ORNITHOPTER).unwrap();
    let elves = on_battlefield(&e, p0, index::LLANOWAR_ELVES).unwrap();
    let thopter = in_hand(&e, p0, index::ORNITHOPTER).unwrap();
    let bears = in_hand(&e, p0, index::GRIZZLY_BEARS).unwrap();
    assert_eq!(
        spendable_units(e.state(), p0, SpendFor::Ability(thopter_on_board)),
        2,
        "control: a colorless creature is an Eldrazi under the Nexus"
    );
    assert_eq!(
        spendable_units(e.state(), p0, SpendFor::Spell(thopter)),
        2,
        "control: and so is a colorless creature spell"
    );
    assert_eq!(
        spendable_units(e.state(), p0, SpendFor::Ability(elves)),
        0,
        "an Eldrazi, but green: its abilities are not paid for with this mana"
    );
    assert_eq!(
        spendable_units(e.state(), p0, SpendFor::Spell(bears)),
        0,
        "a green creature spell is an Eldrazi under the Nexus, but not colorless"
    );
}
