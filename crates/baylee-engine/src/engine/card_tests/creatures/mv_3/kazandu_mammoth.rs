//! `cards/creatures/mv_3/kazandu_mammoth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The back half of Kazandu Mammoth // Kazandu Valley: "This land enters
/// tapped. {T}: Add {G}."
///
/// CR 712.12 lets a player playing a modal double-faced card as a land
/// choose one of its faces that is a land, and this card prints exactly one
/// — so the choice has a single answer and the engine takes it rather than
/// asking. What arrives is therefore the Valley and not the Elephant, and it
/// arrives tapped: `enter_modifiers` are printed on the *back* face here,
/// and a reader that took `faces[0]` would find none and let the land make
/// mana the turn it landed, which is the fault Glasspool Shore had. So the
/// turn it enters it is offered nothing at all, and the {G} is there a turn
/// later — the only reading that shows the back face brought its own ability
/// rather than the Elephant's trigger.
#[test]
fn kazandu_valley_is_played_as_the_back_face_and_taps_for_green_a_turn_later() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(37, forest())
        .hand(0, &[kazandu_mammoth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let valley = play_land(&mut engine, p0, kazandu_mammoth());
    assert!(
        !matches!(engine.pending(), Pending::ChooseCastMode { .. }),
        "one face is a land and the other is a creature, so CR 712.12's \
         choice has one answer and is never put to the player",
    );
    let object = engine
        .state()
        .object(valley)
        .expect("the land is on the battlefield");
    assert_eq!(object.face_index, 1, "the back face is what was played");
    assert_eq!(
        engine.state().names.get(object.characteristics().name),
        "Kazandu Valley",
    );
    let types = object.characteristics().types;
    assert!(types.contains(TypeSet::LAND), "and it is a land");
    assert!(
        !types.contains(TypeSet::CREATURE),
        "and nothing of the 3/3 Elephant on the other side came with it",
    );
    assert!(
        entered_tapped(&engine, valley),
        "\"This land enters tapped\"",
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a land drop hands priority straight back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        legal.abilities.iter().all(|(id, _)| *id != valley),
        "and a tapped permanent cannot pay {{T}}, so the Valley is offered \
         nothing on the turn it arrived — which is what that sentence costs",
    );

    // A turn each way. The untap step is the earliest this land could ever
    // make mana, which is the other half of the same sentence.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, valley),
        "and it untapped in its controller's own untap step",
    );
    // By printed index, and deliberately not through `legal.mana_abilities`:
    // that list is the CR 305.6 shortcut for a basic land type plus whatever
    // a continuous effect granted, and Kazandu Valley prints no subtype at
    // all — its `{T}: Add {G}` is an ability of its own back face.
    activate(&mut engine, p0, kazandu_mammoth(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "\"{{T}}: Add {{G}}\" — off the face that prints it, and resolved \
         without the stack (CR 605.3b)",
    );
    assert!(
        is_tapped(&engine, valley),
        "and the {{T}} in its cost was paid by tapping the Valley itself",
    );
}

/// The front half: "Landfall — Whenever a land you control enters, this
/// creature gets +2/+2 until end of turn."
///
/// The land that lands is the card's *other* copy, played as Kazandu Valley,
/// so one printing plays both of its own halves. Landfall is an ability word
/// with no rules meaning of its own (CR 207.2c), which means everything here
/// has to be the ordinary trigger machinery: the ability waits on the stack
/// until a player would next receive priority (CR 603.3) instead of pumping
/// the moment the land arrives, `Filter::This` resolves to the Elephant
/// rather than to the land that came in, and the +2/+2 is gone when the turn
/// is (CR 514.2) — a duration left at "while on the battlefield" would leave
/// a 5/5 standing on the opponent's turn, and no test that only counted the
/// pump would say so.
#[test]
fn a_land_you_control_entering_pumps_kazandu_mammoth_until_the_turn_ends() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[kazandu_mammoth(), kazandu_mammoth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, kazandu_mammoth());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p0, kazandu_mammoth()).is_some()
    });
    // Captured *before* the second copy is played: both halves are the same
    // `CardIndex`, so afterwards `on_battlefield` could answer with the land.
    let elephant = on_battlefield(&engine, p0, kazandu_mammoth()).expect("the Elephant resolved");
    assert_eq!(
        pt(&engine, elephant),
        (3, 3),
        "cast off three Forests as the printed 3/3",
    );

    let valley = play_land(&mut engine, p0, kazandu_mammoth());
    assert_eq!(
        engine
            .state()
            .object(valley)
            .expect("the land is on the battlefield")
            .face_index,
        1,
        "the land that triggers it is this card's own back face",
    );
    assert!(
        !stack_is_empty(&engine),
        "landfall is a triggered ability and waits on the stack (CR 603.3)",
    );
    assert_eq!(
        pt(&engine, elephant),
        (3, 3),
        "so the Elephant is untouched while it sits there",
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, elephant),
        (5, 5),
        "\"this creature gets +2/+2\" — *this* creature, the trigger's own \
         source, and not the land that entered",
    );

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, elephant),
        (3, 3),
        "\"until end of turn\" ends in that turn's cleanup step (CR 514.2)",
    );
}

/// The narrow word in the same sentence: "whenever a land **you control**
/// enters".
///
/// Every land that enters is somebody's, so a filter narrowed to your own and
/// a filter over every land agree on every other board there is — this is the
/// one scenario that tells them apart, and Kazandu Mammoth is printed with the
/// narrow one. Both directions are struck: the opponent's land really reaches
/// the battlefield, so a silent 3/3 cannot be a land drop that never happened,
/// and nothing goes on the stack at all.
#[test]
fn an_opponents_land_leaves_kazandu_mammoth_the_three_three_it_prints() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(53, forest())
        .battlefield(0, &[kazandu_mammoth()])
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elephant = on_battlefield(&engine, p0, kazandu_mammoth()).expect("the Elephant is seated");
    assert_eq!(pt(&engine, elephant), (3, 3), "a 3/3 to start with");

    reach_their_main_phase(&mut engine, p1);
    let theirs = play_land(&mut engine, p1, forest());
    assert!(
        engine
            .state()
            .object(theirs)
            .is_some_and(|o| o.zone == Zone::Battlefield && o.controller == p1),
        "the opponent's Forest really did enter, under the opponent",
    );
    assert!(
        stack_is_empty(&engine),
        "and no landfall trigger was put on the stack for it",
    );
    assert_eq!(
        pt(&engine, elephant),
        (3, 3),
        "\"a land you control\" — the Elephant's controller played none of it",
    );
}
