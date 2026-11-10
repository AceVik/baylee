//! `cards/enchantments/mv_6/dragonback_assault.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dragonback Assault is `{3}{G}{U}{R}` for two printed sentences: an
/// enters-trigger that deals 3 damage to each creature and each planeswalker,
/// and landfall — a 4/4 red Dragon with flying whenever a land its
/// controller's controls enters. The damage is read off three bodies on one
/// board: a 3/3 of mine that has to die to three points, a 1/1 across the
/// table that has to die to the same three, and a 7/5 of mine that has to
/// survive them, which is what tells damage to each creature from a destroy
/// and three points from one. The landfall half is then played rather than
/// read: one of my lands enters and a Dragon arrives, while the same land drop
/// by the other seat a turn later leaves the count where it was.
///
/// "And each planeswalker" is read off Karn, the Great Creator across the
/// table: printed loyalty 5, so three damage leaves him standing at 2 with no
/// damage marked (CR 120.3c) — a 3-loyalty walker would be in the graveyard
/// before anything could look at it.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn dragonback_assault_shoots_each_creature_and_makes_a_dragon_for_a_land_of_yours() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        // Six lands, which is exactly {3}{G}{U}{R}, plus the two bodies the
        // damage is measured against.
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                island(),
                island(),
                mountain(),
                katara_the_fearless(),
                a_seven_five(),
            ],
        )
        .battlefield(1, &[llanowar_elves(), karn_the_great_creator()])
        // The Assault itself, and the land that is the landfall half.
        .hand(0, &[dragonback_assault(), forest()])
        // A land of their own, for the control at the end.
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches a main phase of its own"
    );

    let katara = on_battlefield(&engine, p0, katara_the_fearless()).expect("Katara is out");
    let wurm = on_battlefield(&engine, p0, a_seven_five()).expect("the big body is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let karn = on_battlefield(&engine, p1, karn_the_great_creator()).expect("their Karn is out");
    let loyalty = |engine: &Engine<RegistryLookup>| {
        engine
            .state()
            .object(karn)
            .expect("Karn is an object")
            .counters
            .get(baylee_cards_dsl::CounterKind::Loyalty)
    };
    assert_eq!(loyalty(&engine), 5, "Karn enters with his printed loyalty");
    assert_eq!(
        pt(&engine, katara),
        (3, 3),
        "a printed 3/3 before any damage"
    );
    assert_eq!(
        pt(&engine, wurm),
        (7, 5),
        "and a body three points cannot kill"
    );
    assert_eq!(pt(&engine, elf), (1, 1), "with a 1/1 across the table");
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "arriving is not a land entering: nothing has been made yet"
    );

    // Mana into the pool before the cast is claimed: `can_afford` reads the
    // pool and not the untapped lands. Both creatures are named as the
    // objects kept back, so the six on the pool are the six lands.
    tap_mana_where(&mut engine, p0, |id| id != katara && id != wurm);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "three Forests, two Islands and a Mountain, and neither creature paid in"
    );
    cast_with_floating(&mut engine, p0, dragonback_assault());
    // Let the spell resolve; the enters-trigger it puts on the stack behind
    // itself is answered by the same walk.
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, dragonback_assault()).is_some(),
        "the enchantment resolved onto the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, katara_the_fearless()).is_some(),
        "3 damage on a printed 3/3 is lethal (CR 704.5g), and it is my own \
         creature: \"each creature\" reaches this side of the table too"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "and the opponent's 1/1 died to the same trigger, so the damage is not \
         \"each creature you control\""
    );
    assert_eq!(
        pt(&engine, wurm),
        (7, 5),
        "while the 7/5 is still standing under the same three points: the \
         trigger damages each creature rather than destroying them"
    );
    assert_eq!(
        on_battlefield(&engine, p1, karn_the_great_creator()),
        Some(karn),
        "Karn survives three points of his five"
    );
    assert_eq!(loyalty(&engine), 2, "\"and each planeswalker\": 5 - 3");
    assert_eq!(
        engine.state().object(karn).expect("Karn").damage,
        0,
        "damage to a planeswalker removes loyalty and is not marked (CR 120.3c)"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and still no Dragon — only a land entering makes one"
    );

    // The landfall half. The land is played rather than seated, because the
    // trigger watches an entry and not a board.
    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, |e| !tokens_of(e, p0).is_empty());

    let dragons = tokens_of(&engine, p0);
    assert_eq!(dragons.len(), 1, "one land, one Dragon");
    let dragon = dragons[0];
    assert_eq!(pt(&engine, dragon), (4, 4), "the printed 4/4 body");
    assert!(
        keywords(&engine, dragon).contains(KeywordSet::FLYING),
        "and the flying the token is printed with"
    );
    assert!(
        types(&engine, dragon).contains(TypeSet::CREATURE),
        "it is a creature token: {:?}",
        types(&engine, dragon)
    );
    let printed = engine
        .state()
        .object(dragon)
        .expect("the Dragon is an object")
        .token
        .expect("a token and not a card that arrived from somewhere");
    assert!(
        printed.colors.contains(baylee_core::color::Color::Red),
        "a 4/4 *red* Dragon"
    );

    // The other half of "a land *you* control": the same land drop by the
    // other seat leaves the count where it was.
    reach_their_main_phase(&mut engine, p1);
    let their_land = in_hand(&engine, p1, forest()).expect("p1 is holding a land of its own");
    engine
        .apply(p1, PlayerAction::PlayLand { card: their_land })
        .expect("a land drop in their own main phase is legal");
    pass_until(&mut engine, |e| at_rest(e, p1));
    assert_eq!(
        tokens_of(&engine, p0).len(),
        1,
        "landfall watches *your* lands: the opponent's land entering made no Dragon"
    );
}
