//! A static ability on one permanent reaching another, and exactly how far it reaches: hexproof from a Privileged Position or a pair of Swiftfoot Boots, shroud from Lightning Greaves, indestructible from a Darksteel Forge, and the two points of power a Sword of Hearth and Home hands the creature wearing it, which Esper Sentinel's `{X}` tax then reads back off it. Every grant is asked from both seats, because hexproof stops opponents and shroud stops everybody (CR 702.11b, CR 702.18b) and a test that only ever asks the opponent cannot tell the two keywords apart — and the grant that arrives through an *attachment* is asked beside an unequipped neighbour, which is the only thing that says the filter did not reach the whole board. Indestructible is read off the battlefield rather than off the options list, because it never stopped the targeting (CR 702.12b). An ability a copy brought with it is not a grant; that is `copied_abilities`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// Privileged Position ("**Other** permanents **you control** have
/// hexproof") against an opponent's Vindicate.
///
/// Three answers on one board, and the card is wrong if any of them flips.
/// Your Elf is hidden, because that is what the grant is for. Their Wizard
/// is not, because a grant that reached across the table would read exactly
/// the same on a board with one creature on it. And the Position **itself**
/// is not, because it says "other" — the word that makes the enchantment
/// the one thing an opponent can answer it with, and a filter written
/// `ControlledByYou` alone would quietly protect it.
#[test]
fn a_privileged_position_hides_your_board_from_them_but_not_itself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = a_table_under_a_privileged_position();
    // Not `reach_main_phase`: reaching the *other* seat's turn crosses a
    // combat phase, and that helper answers priority and nothing else.
    reach_their_main_phase(&mut engine, p1);

    let position = on_battlefield(&engine, p0, privileged_position()).expect("your enchantment");
    let yours = on_battlefield(&engine, p0, llanowar_elves()).expect("your elves");
    let theirs = on_battlefield(&engine, p1, snapcaster_mage()).expect("their mage");

    cast_from_hand(&mut engine, p1, vindicate());
    let options = target_options(&engine);
    assert!(
        !options.contains(&yours),
        "their Vindicate may not target a creature the Position gave \
         hexproof (CR 702.11b): {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "the grant is to permanents *you* control, so their own creature is \
         still a legal target: {options:?}"
    );
    assert!(
        options.contains(&position),
        "the Position says \"other\" and does not protect itself: {options:?}"
    );
}

/// The same board, the same spell, cast by the seat that owns the grant.
///
/// Hexproof is "can't be the target of spells or abilities *your opponents*
/// control" (CR 702.11b), so this is the half that a `Filter::ControlledBy`
/// mistake would take away. A creature under a Privileged Position that its
/// own controller could no longer target would break every aura, every
/// pump spell and every equip in the deck built around it — and no test
/// asking the opponent's question would notice.
#[test]
fn your_own_removal_still_reaches_the_creature_you_gave_hexproof() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = a_table_under_a_privileged_position();
    reach_main_phase(&mut engine, p0);

    let yours = on_battlefield(&engine, p0, llanowar_elves()).expect("your elves");
    let theirs = on_battlefield(&engine, p1, snapcaster_mage()).expect("their mage");

    cast_from_hand(&mut engine, p0, vindicate());
    let options = target_options(&engine);
    assert!(
        options.contains(&yours),
        "hexproof stops opponents only, so your own spell still sees your \
         own creature: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "and nothing about the Position was ever between you and their \
         board: {options:?}"
    );
}

/// Lightning Greaves ("Equipped creature has haste and **shroud**") and the
/// controller's own Vindicate.
///
/// The counterpart to the two above, on the distinction the two keywords
/// exist for: shroud is "can't be the target of spells or abilities"
/// (CR 702.18b) full stop, so the same seat that granted it is refused —
/// which is the whole reason a player equips Greaves and then complains
/// they cannot aura the creature. The unequipped creature beside it is the
/// bystander, and it says the refusal came from the attachment rather than
/// from the spell.
#[test]
fn greaves_hide_the_creature_they_are_on_from_you_too() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(81, forest())
        .battlefield(
            0,
            &[
                lightning_greaves(),
                llanowar_elves(),
                snapcaster_mage(),
                plains(),
                swamp(),
                forest(),
            ],
        )
        .hand(0, &[vindicate()])
        .battlefield(1, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let greaves = on_battlefield(&engine, p0, lightning_greaves()).expect("the Greaves");
    let equipped = on_battlefield(&engine, p0, llanowar_elves()).expect("your elves");
    let bystander = on_battlefield(&engine, p0, snapcaster_mage()).expect("your mage");
    let across = on_battlefield(&engine, p1, ondu_cleric()).expect("their cleric");

    let equip = offered_ability(&engine, greaves).expect("equip {0} is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: greaves,
                ability_index: equip,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![equipped],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().object(equipped).is_some_and(|o| {
            o.characteristics()
                .keywords
                .contains(baylee_cards_dsl::KeywordSet::SHROUD)
        })
    });

    cast_from_hand(&mut engine, p0, vindicate());
    let options = target_options(&engine);
    assert!(
        !options.contains(&equipped),
        "shroud refuses its own controller as well (CR 702.18b): {options:?}"
    );
    assert!(
        options.contains(&bystander),
        "the creature beside it is wearing nothing: {options:?}"
    );
    assert!(
        options.contains(&greaves),
        "the Equipment grants to the creature it is attached to, never to \
         itself: {options:?}"
    );
    assert!(
        options.contains(&across),
        "and the board across the table is untouched by any of it: {options:?}"
    );
}

/// The Forge under their removal: my artifact survives a spell that resolved.
#[test]
fn a_darksteel_forge_keeps_your_artifact_through_their_removal() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = a_table_under_a_darksteel_forge(84);
    reach_their_main_phase(&mut engine, p1);

    let mine = on_battlefield(&engine, p0, liquimetal_coating()).expect("my coating");
    assert!(
        vindicated(&mut engine, p1, mine),
        "\"artifacts you control have indestructible\" — destruction does \
         nothing to it (CR 702.12b)"
    );
}

/// The counter-probe on the same board: the grant is to **artifacts** I
/// control, not to everything I control.
///
/// Its own game rather than a second spell in the one above, because the
/// interesting failure is a Forge that saved the Elf too — and a test that
/// had already spent the turn's mana could not ask.
#[test]
fn the_forge_is_not_a_shield_over_the_rest_of_your_board() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = a_table_under_a_darksteel_forge(85);
    reach_their_main_phase(&mut engine, p1);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my elves");
    assert!(
        !vindicated(&mut engine, p1, elf),
        "the Elf is not an artifact and the Forge never mentioned it"
    );
}

/// And the direction across the table: their artifact is not mine, so my own
/// Forge does not save it from my own Vindicate.
#[test]
fn the_forge_does_not_reach_the_artifact_across_the_table() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = a_table_under_a_darksteel_forge(86);
    reach_main_phase(&mut engine, p0);

    let theirs = on_battlefield(&engine, p1, liquimetal_coating()).expect("their coating");
    assert!(
        !vindicated(&mut engine, p0, theirs),
        "\"artifacts **you** control\" is the whole scope of the grant"
    );
}

/// Esper Sentinel taxes `{X}`, where X is **its own power** — so a Sword of
/// Hearth and Home on it turns a `{1}` tax into `{3}`.
///
/// The pair is the evidence, not either half. The card was written as a flat
/// `mana: 1`, which is the right answer for an unequipped 1/1 and stays the
/// right answer forever: the unequipped test below passes against the wrong
/// card and the equipped one does not, so it is the equipped number that
/// says the amount is being read off the creature at all.
#[test]
fn a_sword_on_the_sentinel_raises_the_tax_it_asks_for() {
    assert_eq!(
        the_tax_the_sentinel_asks_for(93, true),
        3,
        "a 1/1 wearing +2/+2 taxes {{3}}"
    );
}

#[test]
fn an_unequipped_sentinel_asks_for_its_printed_one() {
    assert_eq!(
        the_tax_the_sentinel_asks_for(94, false),
        1,
        "the same card with nothing on it taxes {{1}}"
    );
}

/// Swiftfoot Boots ("Equipped creature has **hexproof** and haste") against
/// the opponent's Vindicate.
///
/// The Boots are Lightning Greaves' shape with the other keyword in it, and
/// the pair is why the engine keeps two bits rather than one: shroud refuses
/// everybody, hexproof refuses opponents (CR 702.11b). Both grants arrive
/// through an *attachment*, which is the part a static filter can get wrong
/// in a way the Position's own tests could never see — `AttachedToBySource`
/// reaching every creature you control reads exactly like a working Equipment
/// while one creature is wearing it. The unequipped mage beside it is the
/// bystander that says otherwise.
#[test]
fn the_boots_hide_the_creature_they_are_on_from_the_other_seat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(97, forest())
        .battlefield(
            0,
            &[
                swiftfoot_boots(),
                llanowar_elves(),
                snapcaster_mage(),
                forest(),
            ],
        )
        .battlefield(1, &[ondu_cleric(), plains(), swamp(), forest()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let boots = on_battlefield(&engine, p0, swiftfoot_boots()).expect("the Boots");
    let equipped = on_battlefield(&engine, p0, llanowar_elves()).expect("your elves");
    let bystander = on_battlefield(&engine, p0, snapcaster_mage()).expect("your mage");

    // Equip is {1} here rather than the Greaves' {0}, so the mana has to be
    // floating before the ability is offered at all.
    tap_mana_except(&mut engine, p0, boots);
    let equip = offered_ability(&engine, boots).expect("equip {1} is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: boots,
                ability_index: equip,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![equipped],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().object(equipped).is_some_and(|o| {
            o.characteristics()
                .keywords
                .contains(baylee_cards_dsl::KeywordSet::HEXPROOF)
        })
    });
    assert!(
        engine.state().object(equipped).is_some_and(|o| o
            .characteristics()
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::HASTE)),
        "the Boots grant both halves of their sentence"
    );

    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, vindicate());
    let options = target_options(&engine);
    assert!(
        !options.contains(&equipped),
        "hexproof is a refusal to the seat across the table: {options:?}"
    );
    assert!(
        options.contains(&bystander),
        "and it stops at the creature the Boots are on: {options:?}"
    );
    assert!(
        options.contains(&boots),
        "the Equipment grants to what it is attached to, never to itself: \
         {options:?}"
    );
}

/// The same Boots, the same creature, and the seat that put them there.
///
/// This is the half that tells the two keywords apart, and the one a
/// Greaves-shaped copy-paste would take away: a creature its own controller
/// could no longer target is a creature that can never be equipped again,
/// auraed or pumped, and every assertion in the test above would still pass.
#[test]
fn your_own_spell_still_reaches_the_creature_wearing_your_boots() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(98, forest())
        .battlefield(
            0,
            &[
                swiftfoot_boots(),
                llanowar_elves(),
                snapcaster_mage(),
                plains(),
                plains(),
                swamp(),
                swamp(),
                forest(),
            ],
        )
        .hand(0, &[vindicate()])
        .battlefield(1, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let boots = on_battlefield(&engine, p0, swiftfoot_boots()).expect("the Boots");
    let equipped = on_battlefield(&engine, p0, llanowar_elves()).expect("your elves");

    // Generic mana is spent white first, so the extra Plains and Swamp are
    // what leave Vindicate's own {1}{W}{B} payable after the equip.
    tap_mana_except(&mut engine, p0, boots);
    let equip = offered_ability(&engine, boots).expect("equip {1} is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: boots,
                ability_index: equip,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![equipped],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().object(equipped).is_some_and(|o| {
            o.characteristics()
                .keywords
                .contains(baylee_cards_dsl::KeywordSet::HEXPROOF)
        })
    });

    cast_from_hand(&mut engine, p0, vindicate());
    let options = target_options(&engine);
    assert!(
        options.contains(&equipped),
        "hexproof is not shroud: the seat that granted it may still point \
         at the creature (CR 702.11b): {options:?}"
    );
}
