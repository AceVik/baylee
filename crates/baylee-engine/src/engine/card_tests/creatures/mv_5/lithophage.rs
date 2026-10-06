//! `cards/creatures/mv_5/lithophage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "b4eb3d7e-a234-4ed5-8611-fcb818686fcf"

/// Lithophage — {3}{R}{R} — Creature — Insect 7/7 — prints one sentence and
/// nothing else: "At the beginning of your upkeep, sacrifice this creature
/// unless you sacrifice a Mountain."
///
/// The whole card is that "unless", and both of its outcomes are the engine's
/// answer rather than the card's, so both are played. The first board casts
/// the 7/7 off exactly five Mountains, walks a full turn into its controller's
/// next upkeep, and reads the price menu where the engine builds it: the five
/// Mountains this seat controls, with the Mountain across the table declined
/// by "you control" — paying one of them is then what keeps the 7/7 standing,
/// which an engine that took the price as a formality could not show. The
/// second board is the control the first one cannot be: the same creature on a
/// battlefield with no Mountain anywhere, where there is no price to name,
/// nothing to ask about and only the printed fallback left.
#[test]
#[allow(clippy::too_many_lines)]
fn lithophage_eats_a_mountain_at_its_upkeep_or_eats_itself_when_there_is_none() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    // ---- the half that has a Mountain to give up ------------------------

    let mut engine = Duel::new(SEED, basic_forest())
        .battlefield(0, &[mountain(); 5])
        // A Mountain across the table: "a Mountain" is read as one of the
        // seat's own, and a same-card bystander is the only thing that says so.
        .battlefield(1, &[mountain()])
        .hand(0, &[lithophage()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Five Mountains are the whole cost of {3}{R}{R}, and the board holds no
    // other mana source at all.
    cast_from_hand(&mut engine, p0, lithophage());
    pass_until(&mut engine, stack_is_empty);
    let bug = on_battlefield(&engine, p0, lithophage()).expect("the 7/7 resolved");
    assert_eq!(pt(&engine, bug), (7, 7), "the body the card prints");
    let theirs = on_battlefield(&engine, p1, mountain()).expect("their Mountain is out");
    assert_eq!(
        mine(&engine, p0, mountain(), Zone::Battlefield).len(),
        5,
        "and the five that paid for it are standing untapped"
    );

    // One full turn cycle: the upkeep the sentence is about is this seat's
    // next one, past the combat steps of both players. The printed "unless"
    // asks no yes-or-no question first: the seat is shown what may pay, and
    // naming nothing is how it declines.
    let mut price: Vec<ObjectId> = Vec::new();
    for _ in 0..400 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(player, p0, "the seat that pays the price names it");
                assert_eq!(
                    prompt,
                    ChoicePrompt::CostSacrifice,
                    "a cost and not a search, which is all a client has to tell apart"
                );
                assert_eq!(
                    (min, max),
                    (0, 1),
                    "one Mountain or none — naming nothing is declining the price"
                );
                assert!(
                    !options.contains(&theirs),
                    "`CR 701.21a`: the Mountain across the table is not this \
                     seat's to sacrifice: {options:?}"
                );
                price = options;
                break;
            }
            other => panic!("unexpected on the way to the upkeep: {other:?}"),
        }
    }
    assert_eq!(
        price.len(),
        5,
        "the five Mountains this seat controls are the whole of the price: {price:?}"
    );

    let paid = price[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![paid],
            },
        )
        .expect("the Mountain the question offered pays the price");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(paid).map(|o| o.zone),
        Some(Zone::Graveyard),
        "the Mountain that was named is the one that went to the graveyard"
    );
    assert_eq!(
        mine(&engine, p0, mountain(), Zone::Battlefield).len(),
        4,
        "exactly one Mountain paid the price, not the whole board of them"
    );
    assert!(
        on_battlefield(&engine, p0, lithophage()).is_some(),
        "with the price paid the 7/7 stays — a trigger that collected the \
         Mountain and sacrificed the creature anyway would fail right here"
    );

    // ---- the half with no Mountain anywhere -----------------------------

    let mut bare = Duel::new(SEED, basic_forest())
        .battlefield(0, &[lithophage(), basic_forest()])
        .start();
    keep_mulligans(&mut bare);
    let mut asked = false;
    for _ in 0..400 {
        if in_graveyard(&bare, p0, lithophage()).is_some() {
            break;
        }
        match bare.pending().clone() {
            Pending::Priority { player, .. } => {
                bare.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                bare.apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                bare.apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            // Should never arrive: there is no Mountain to name, so the price
            // has no answer to ask about. Answered "no" so that a question
            // this engine does ask still leaves the creature dying, and the
            // assertion below is what notices it was asked at all.
            Pending::YesNo { player, .. } => {
                asked = true;
                bare.apply(player, PlayerAction::YesNo(false)).unwrap();
            }
            other => panic!("unexpected on the bare board: {other:?}"),
        }
    }

    assert!(
        in_graveyard(&bare, p0, lithophage()).is_some(),
        "with no Mountain to give up the printed \"unless\" does what it says: \
         the 7/7 sacrifices itself"
    );
    assert!(
        !asked,
        "and there was nothing to ask about — an unpayable price is no \
         question at all rather than a question with an empty menu"
    );
}
