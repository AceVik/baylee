//! `cards/creatures/mv_5/wind_spirit.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "d4c22b68-c4cb-447a-97df-431e2f1e31f2"

/// Wind Spirit — {4}{U} — Creature — Elemental Spirit, 3/2, printing two
/// keywords and nothing else: flying and menace.
///
/// Keywords printed on a card are only worth what the layer projection says
/// they are, so both halves of the sentence are read off the permanent after
/// it has resolved rather than off the card file: a 3/2 body, flying, and
/// menace — a keyword the combat step consumes as a blocking restriction
/// rather than as a line of text. The Elf beside it is the control, because
/// "flying and menace" is a claim about the Spirit and not about the seat:
/// it stands untapped and keywordless on the same battlefield, and the five
/// Islands that paid {4}{U} leave an empty pool behind them.
#[test]
fn wind_spirit_lands_as_a_three_two_flying_menace_creature_and_arms_no_other() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[wind_spirit()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is on the table");
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FLYING),
        "the bystander is a printed 1/1 with no text before anything is cast"
    );

    // Five Islands pay {4}{U}, and the Elf is named as the printing kept back:
    // it is the creature both keyword readings are taken against, and
    // `tap_all_mana` would have drunk its own `{T}: Add {G}` as well (#159).
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Islands tapped and no Elf: five mana for a five-mana creature"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        5,
        "one of them is the blue the printed {{U}} asks for"
    );

    cast_with_floating(&mut engine, p0, wind_spirit());
    pass_until(&mut engine, stack_is_empty);

    let spirit = on_battlefield(&engine, p0, wind_spirit()).expect("the Spirit resolved");
    assert!(
        types(&engine, spirit).contains(TypeSet::CREATURE),
        "it arrives as the creature the card prints"
    );
    assert_eq!(
        pt(&engine, spirit),
        (3, 2),
        "3/2: a body read through the layers and not a pair of card fields"
    );

    let granted = keywords(&engine, spirit);
    assert!(
        granted.contains(KeywordSet::FLYING),
        "\"Flying\" reaches the permanent the spell became"
    );
    assert!(
        granted.contains(KeywordSet::MENACE),
        "\"Menace\" is a keyword on the projected characteristics, which is \
         what the blocking offer will read it from"
    );
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FLYING)
            && !keywords(&engine, elf).contains(KeywordSet::MENACE),
        "the keywords belong to the Spirit and are not a board-wide grant"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{4}}{{U}} came out of the pool"
    );
}
