//! `cards/creatures/mv_5/umara_wizard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Umara Wizard ({4}{U}, 4/3 Merfolk Wizard): "Whenever you cast an instant,
/// sorcery, or Wizard spell, this creature gains flying until end of turn."
///
/// The printed sentence names three kinds of spell, and the one that is easy
/// to get wrong is the third: "Wizard spell" reads the *tribe of a card on the
/// stack*, which is a characteristic nothing else in this scenario asks about.
/// So it needs a control beside it, or "flying appeared" would be equally true
/// of a trigger that fired on any spell at all. Llanowar Elves is that
/// control, and Viscera Seer is the same shape one tribe over: both are
/// one-mana creature spells, and the only difference between them is that the
/// Seer's printing says Vampire **Wizard**.
///
/// The Wizard itself is cast rather than seeded, for two reasons. The front
/// face is the face a player chooses when casting a modal double-faced card
/// (CR 712.11b), so a seeded permanent would never have gone that way at all;
/// and its own cast is the first thing that could wrongly fire the ability —
/// the card it was cast from *is* a Wizard spell — which it must not, because
/// the ability only exists while the permanent is on the battlefield. The 4/3
/// that lands without flying is that assertion.
#[test]
fn an_umara_wizard_takes_flight_for_a_wizard_spell_and_stays_down_for_an_elf() {
    let p0 = PlayerId::new(0);
    // Five Islands pay {4}{U} exactly, which leaves the Forest and the Swamp
    // untouched for the two creature spells below: each of them is paid for
    // with the mana of its own colour and nothing else.
    let mut board = vec![island(); 5];
    board.extend_from_slice(&[forest(), swamp()]);
    let mut engine = Duel::new(47, island())
        .battlefield(0, &board)
        .hand(0, &[umara_wizard(), llanowar_elves(), viscera_seer()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);

    for source in all_on_battlefield(&engine, p0, island()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let card = in_hand(&engine, p0, umara_wizard()).expect("the card is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "the front face of a modal double-faced card is cast like any other \
         creature card (CR 712.11b)",
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("the creature face is cast");
    pass_until(&mut engine, stack_is_empty);

    let wizard = on_battlefield(&engine, p0, umara_wizard()).expect("the Wizard resolved");
    assert_eq!(pt(&engine, wizard), (4, 3), "the printed body arrived");
    assert!(
        !keywords(&engine, wizard).contains(KeywordSet::FLYING),
        "its own cast was a Wizard spell and gave it nothing: the ability is \
         on the permanent, and the permanent was on the stack",
    );

    // "An instant, sorcery, or Wizard spell" — an Elf Druid is none of them.
    for source in all_on_battlefield(&engine, p0, forest()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let elf = in_hand(&engine, p0, llanowar_elves()).expect("the Elf is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: elf })
        .expect("the Elf is cast");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !keywords(&engine, wizard).contains(KeywordSet::FLYING),
        "a creature spell that is not a Wizard leaves it on the ground — if \
         this fires, the trigger is reading \"a spell\" and the three printed \
         kinds are decoration",
    );

    // Viscera Seer is a Vampire Wizard: one mana, one tribe of difference.
    for source in all_on_battlefield(&engine, p0, swamp()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let seer = in_hand(&engine, p0, viscera_seer()).expect("the Seer is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: seer })
        .expect("the Seer is cast");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        keywords(&engine, wizard).contains(KeywordSet::FLYING),
        "\"or Wizard spell\" reads the tribe of the card on the stack, which \
         is the only thing the Seer had that the Elf did not",
    );
}

/// The other two words in the same sentence: "**you** cast", and "until end of
/// turn".
///
/// Dark Ritual is the instant clause in its smallest form — no targets, no
/// choices, nothing on the board to read — and both halves of this scenario
/// hang off *both* seats holding one. The grant is taken on its controller's
/// turn, and then two things have to be true that a careless continuous effect
/// gets wrong: it is gone on the next turn, because the cleanup step ends
/// every "until end of turn" effect (CR 514.2), and the opponent casting the
/// identical instant on that turn brings nothing back, because
/// `Filter::ControlledByYou` is the whole of "you cast".
///
/// Neither half stands alone. Without the second, the first would be satisfied
/// by an effect that expired for some reason of its own; without the first,
/// the second would be satisfied by an effect that had simply never ended.
#[test]
fn umara_wizards_flying_ends_at_cleanup_and_never_comes_off_an_opponents_instant() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(53, forest())
        .battlefield(0, &[umara_wizard(), swamp()])
        .battlefield(1, &[swamp()])
        .hand(0, &[dark_ritual()])
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);
    let wizard = on_battlefield(&engine, p0, umara_wizard()).expect("the Wizard stands");
    assert!(
        !keywords(&engine, wizard).contains(KeywordSet::FLYING),
        "nothing has been cast yet, and the printing grants no flying of its own",
    );

    cast_from_hand(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        keywords(&engine, wizard).contains(KeywordSet::FLYING),
        "Dark Ritual is an Instant, the first of the three kinds the sentence \
         names",
    );

    let granted_on = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > granted_on
            && e.state().turn.active == p1
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    assert!(
        !keywords(&engine, wizard).contains(KeywordSet::FLYING),
        "\"until end of turn\": the cleanup step of the turn it was granted on \
         ended the effect (CR 514.2)",
    );

    // The same card, cast by the other seat, on that seat's own turn.
    cast_from_hand(&mut engine, p1, dark_ritual());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !keywords(&engine, wizard).contains(KeywordSet::FLYING),
        "the sentence says whenever *you* cast — an opponent's instant is not \
         the controller's, so the trigger never fires",
    );
}

/// The middle word of the three, and the one the other two tests never reach:
/// "instant, **sorcery**, or Wizard spell".
///
/// Its absence was the hole worth closing. The Wizard clause is struck by the
/// Viscera Seer above and the instant clause by Dark Ritual, so
/// `Filter::HasType(TypeSet::SORCERY)` could be deleted from the card's filter
/// and every other assertion here would stay green. Vindicate is that word on
/// its own: a Sorcery, no creature type at all, and nothing in common with
/// either of the other two clauses.
///
/// It targets on purpose. A spell becomes cast at the *end* of the casting
/// procedure, and that is where a cast trigger fires (CR 601.2i) — so the
/// question the engine stops on in the middle is a moment at which the trigger
/// must not yet have granted anything, and the reading taken there is the
/// second half of this test rather than a detour around Vindicate's target.
#[test]
fn an_umara_wizard_takes_flight_for_the_sorcery_its_controller_casts() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(71, forest())
        .battlefield(0, &[umara_wizard(), plains(), swamp(), island()])
        .battlefield(1, &[swamp()])
        .hand(0, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);
    let wizard = on_battlefield(&engine, p0, umara_wizard()).expect("the Wizard stands");
    assert!(
        !keywords(&engine, wizard).contains(KeywordSet::FLYING),
        "nothing has been cast yet, and the 4/3 prints no flying of its own",
    );

    // {1}{W}{B} off the Plains, the Swamp and the Island, which is every mana
    // source p0 has — the Wizard itself makes none.
    cast_from_hand(&mut engine, p0, vindicate());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"destroy target permanent\" is asked mid-cast: {:?}",
            engine.pending()
        )
    };
    let theirs = on_battlefield(&engine, p1, swamp()).expect("the other seat has a Swamp");
    assert!(
        options.contains(&theirs),
        "a land across the table is a permanent: {options:?}",
    );
    assert!(
        !keywords(&engine, wizard).contains(KeywordSet::FLYING),
        "the spell is not cast until the procedure finishes (CR 601.2i), so \
         nothing has triggered while the target is still being chosen",
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("a permanent is a legal answer to \"target permanent\"");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        keywords(&engine, wizard).contains(KeywordSet::FLYING),
        "Vindicate is a Sorcery and nothing else the sentence names — no \
         instant, no Wizard — so this is the only assertion in the pool that \
         notices if that clause goes missing",
    );
    assert!(
        on_battlefield(&engine, p1, swamp()).is_none(),
        "and the sorcery that triggered it did what it prints",
    );
}

/// Umara Skyfalls, the back face: "This land enters tapped." and "{T}: Add
/// {U}."
///
/// A player playing a modal double-faced card as a land chooses one of its
/// faces that is a land, and it enters the battlefield with that face up
/// (CR 712.12). Only one of Umara Wizard's two faces is a land, so that choice
/// has exactly one answer and the engine takes it without asking — which is
/// worth striking on its own, because a `ChooseCastMode` with one option here
/// would be a question a client has to draw a button for.
///
/// Three things are then read off the permanent, and every one of them lives
/// on the *back* face's own data rather than on the card's first face: the
/// name and the land type, the enters-tapped modifier, and the mana ability. A
/// reader that asked `faces[0]` — which is how Glasspool Shore once came down
/// untapped — would answer a creature, no modifier and no ability to all three.
#[test]
fn umara_skyfalls_is_the_land_face_enters_tapped_and_then_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(59, forest()).hand(0, &[umara_wizard()]).start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, umara_wizard()).expect("the card is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.lands.contains(&card),
        "a card whose front face is a creature is still a land drop, because \
         its back face is a land (CR 712.12)",
    );

    let land = play_land(&mut engine, p0, umara_wizard());
    assert!(
        !matches!(engine.pending(), Pending::ChooseCastMode { .. }),
        "one of the two faces is a land, so there is nothing to choose \
         between and nothing is asked: {:?}",
        engine.pending()
    );
    let played = engine
        .state()
        .object(land)
        .expect("the land is on the battlefield");
    let types = played.characteristics().types;
    assert_eq!(played.face_index, 1, "it entered with the land face up");
    assert_eq!(
        engine.state().names.get(played.characteristics().name),
        "Umara Skyfalls",
        "and the permanent is that face, name and all (CR 712.8f)",
    );
    assert!(
        types.contains(TypeSet::LAND) && !types.contains(TypeSet::CREATURE),
        "a Land, and not the 4/3 printed on the other side: {types:?}",
    );
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped.\" is printed on the back face, so a \
         modifier read off the front one would have let it make mana at once",
    );

    // Its controller's next turn, so the tap it came down with is spent and
    // what is pressed below is the land's own ability.
    let played_on = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > played_on
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(!is_tapped(&engine, land), "it untapped like any other land");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        unreachable!("the walk above waited for exactly this")
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "\"{{T}}: Add {{U}}\" is the whole of what the land face does, and it \
         is offered: {:?}",
        legal.abilities
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .expect("the land taps for mana");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "one blue, which is the colour the back face prints",
    );
    assert!(is_tapped(&engine, land), "and it paid with its own {{T}}");
}
