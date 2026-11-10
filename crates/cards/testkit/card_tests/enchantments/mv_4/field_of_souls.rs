//! `cards/enchantments/mv_4/field_of_souls.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "4d7a5b14-8fce-41f2-a0d5-fff3d15f41f6"

/// Field of Souls — {2}{W}{W} enchantment: "Whenever a nontoken creature is
/// put into your graveyard from the battlefield, create a 1/1 white Spirit
/// creature token with flying."
///
/// Every death here is dealt by the harness and read back out of a graveyard,
/// so what is measured is the trigger itself: a printed Elf of this seat's
/// dying brings one Spirit, a second one brings a second, and the Elf across
/// the table — a nontoken creature dying where "your graveyard" has to decline
/// it — brings none. The token's own body comes off its token record, and a
/// Spirit token dying afterwards is the `Nontoken` half of the card's filter.
#[test]
fn field_of_souls_makes_a_spirit_for_each_nontoken_creature_of_yours_that_dies() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[field_of_souls()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The enchantment arrives the way the card arrives — {2}{W}{W} out of a
    // pool four Plains and two Elves actually paid into — so the trigger below
    // is not read off a printing that was merely placed on the board.
    cast_from_hand(&mut engine, p0, field_of_souls());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, field_of_souls()).is_some(),
        "the Field resolved onto the battlefield"
    );

    let mine = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(mine.len(), 2, "two Elves of mine and one across the table");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_none(),
        "nothing has died yet"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and nothing has been made yet"
    );

    // (1) A nontoken creature of mine is put into my graveyard.
    kill(&mut engine, mine[0]);
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the creature died into its owner's graveyard, which is the condition \
         the card prints"
    );
    let spirits = tokens_of(&engine, p0);
    assert_eq!(spirits.len(), 1, "one death, one Spirit");
    let spirit = spirits[0];
    assert!(
        types(&engine, spirit).contains(TypeSet::CREATURE),
        "the token the Field makes is a creature: {:?}",
        types(&engine, spirit)
    );
    assert_eq!(pt(&engine, spirit), (1, 1), "the body the token prints");
    assert!(
        keywords(&engine, spirit).contains(KeywordSet::FLYING),
        "and the printed flying reaches the permanent"
    );
    let printed = engine
        .state()
        .object(spirit)
        .expect("the Spirit is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(printed.name, "Spirit");
    assert!(
        printed.colors.contains(baylee_core::color::Color::White),
        "a white Spirit, and not a colourless creature token: {:?}",
        printed.colors
    );

    // (2) A nontoken creature dying where the graveyard is not mine. The
    // object is still in a graveyard afterwards, so "your graveyard" is read
    // on a card and not on a hole in the board.
    kill(&mut engine, theirs);
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the Elf across the table died into its own owner's graveyard"
    );
    assert_eq!(
        tokens_of(&engine, p0).len(),
        1,
        "a creature of theirs dying is no creature of mine, so the Field made \
         nothing for it"
    );

    // (3) The ability is one Spirit per creature, and not one per turn.
    kill(&mut engine, mine[1]);
    assert_eq!(
        tokens_of(&engine, p0).len(),
        2,
        "a second creature of mine dies and a second Spirit arrives"
    );

    // (4) The `Nontoken` half of the filter, on the only token creature this
    // board can hold: a Spirit of the Field's own making. It ceases to exist
    // as it leaves (CR 111.7), so an extra Spirit here would be the trigger
    // firing on it and nothing else.
    let spirits = tokens_of(&engine, p0);
    kill(&mut engine, spirits[0]);
    assert_eq!(
        tokens_of(&engine, p0).len(),
        1,
        "\"nontoken creature\": a Spirit token dying is not one, so no new \
         Spirit was made for it"
    );
}
