//! `cards/instants/mv_2/word_of_command.rs`, played.
//!
//! Word of Command, `{B}{B}`: "Look at target opponent's hand and choose a
//! card from it. You control that player until Word of Command finishes
//! resolving. The player plays that card if able. While doing so, the player
//! can activate mana abilities only if they're from lands that player
//! controls and only if mana they produce is spent to activate other mana
//! abilities of lands the player controls and/or to play that card. If the
//! chosen card is cast as a spell, you control the player while that spell
//! is resolving."
//!
//! The rules it leans on are CR 722 (controlling another player: 722.2 names
//! this card, 722.4 what is visible, 722.5 whose choices and whose resources,
//! 722.5a the controlled player's resources pay) and CR 305 for a land
//! (305.2a and 305.2b the land drop, 305.3 "if it isn't their turn").
//!
//! Three seats, so that "opponent only" and "nobody else sees the hand" are
//! both something a test can ask. Every question is driven by hand: the
//! controller answers the commanded player's questions, and `pass_until`
//! only walks priority and the questions nobody here is testing.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

const USER: PlayerId = PlayerId::new(0);
const OTHER: PlayerId = PlayerId::new(1);
const THIRD: PlayerId = PlayerId::new(2);

// oracle_id = "e8ad3a77-b293-4d69-b080-27ca9f95d443"
fn word_of_command() -> CardIndex {
    card_index("e8ad3a77-b293-4d69-b080-27ca9f95d443")
}

/// `USER` holds Word and two Swamps, `OTHER` the given board and hand,
/// `THIRD` a Forest and a Sol Ring in hand (a card that is not in the
/// commanded player's hand, to be refused). `USER` holds priority in their
/// first main phase.
fn table(board: &[CardIndex], hand: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[word_of_command()])
        .battlefield(1, board)
        .hand(1, hand)
        .battlefield(2, &[forest()])
        .hand(2, &[sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, USER);
    engine
}

/// Lets priority pass until `seat` holds it.
#[track_caller]
fn priority(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == seat),
    );
}

/// Taps every untapped Swamp `USER` has: `{B}{B}` floating for Word.
#[track_caller]
fn tap_swamps(engine: &mut Engine<RegistryLookup>) {
    for source in all_on_battlefield(engine, USER, swamp()) {
        if !is_tapped(engine, source) {
            engine
                .apply(USER, PlayerAction::ActivateManaAbility { source })
                .unwrap();
        }
    }
}

/// The players the question for Word's target offers, and who is asked.
#[track_caller]
fn opponents_offered(engine: &Engine<RegistryLookup>) -> (PlayerId, Vec<PlayerId>) {
    match engine.pending() {
        Pending::ChoosePlayer { player, options } => (*player, options.clone()),
        Pending::ChooseTargets {
            player,
            options,
            player_options,
            ..
        } => {
            assert!(options.is_empty(), "Word targets a player, not an object");
            (*player, player_options.clone())
        }
        pending => panic!("Word's target was expected, got {pending:?}"),
    }
}

/// Answers the open target question with `opponent`, as the decision's
/// actor (the commanded player's questions are answered by the controller).
#[track_caller]
fn aim_word(engine: &mut Engine<RegistryLookup>, opponent: PlayerId) {
    let actor = engine.decision_actor().expect("a question is open");
    match engine.pending().clone() {
        Pending::ChoosePlayer { .. } => engine
            .apply(actor, PlayerAction::ChoosePlayer(opponent))
            .unwrap(),
        Pending::ChooseTargets { .. } => engine
            .apply(
                actor,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![opponent],
                },
            )
            .unwrap(),
        pending => panic!("a target was expected, got {pending:?}"),
    }
}

/// Taps `USER`'s Swamps, casts Word and aims it at `OTHER`.
#[track_caller]
fn cast_word_at_other(engine: &mut Engine<RegistryLookup>) {
    tap_swamps(engine);
    cast_with_floating(engine, USER, word_of_command());
    aim_word(engine, OTHER);
}

/// Lets Word resolve up to its question: the card to choose from the
/// player's hand, asked of the controller. Returns the options.
#[track_caller]
fn reach_the_choice(engine: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, USER, "the controller chooses the card");
    options
}

/// Casts Word at `OTHER` and chooses `card` out of their hand.
#[track_caller]
fn command(engine: &mut Engine<RegistryLookup>, card: CardIndex) -> ObjectId {
    cast_word_at_other(engine);
    reach_the_choice(engine);
    let chosen = in_hand(engine, OTHER, card).expect("the card is in their hand");
    engine
        .apply(
            USER,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .unwrap();
    chosen
}

/// Whether `OTHER` is paying, with `USER` deciding, as Word's window.
fn in_commanded_window(engine: &Engine<RegistryLookup>) -> bool {
    engine
        .payment_window()
        .is_some_and(|(player, _)| player == OTHER)
        && engine.decision_actor() == Some(USER)
}

fn untapped(engine: &Engine<RegistryLookup>, card: CardIndex) -> Vec<ObjectId> {
    all_on_battlefield(engine, OTHER, card)
        .into_iter()
        .filter(|id| !is_tapped(engine, *id))
        .collect()
}

/// The sources whose mana the commanded payment offers right now: the basic
/// lands' shortcut list and every other ability on offer (a Sol Ring, a
/// filter land, are activated as ordinary abilities, not by the shortcut).
#[track_caller]
fn offered_mana_sources(engine: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("a payment window expected, got {:?}", engine.pending())
    };
    let mut sources = legal.mana_abilities.clone();
    sources.extend(legal.abilities.iter().map(|&(source, _)| source));
    sources.sort();
    sources.dedup();
    sources
}

/// Taps each of `sources` for `USER` (who decides for `OTHER`), then closes
/// the window if it is still open.
#[track_caller]
fn pay_with(engine: &mut Engine<RegistryLookup>, sources: &[ObjectId]) {
    for &source in sources {
        engine
            .apply(USER, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    if in_commanded_window(engine) {
        engine.apply(USER, PlayerAction::PassPriority).unwrap();
    }
}

/// Lets everything on the stack resolve.
#[track_caller]
fn settle(engine: &mut Engine<RegistryLookup>) {
    pass_until(engine, stack_is_empty);
}

// ---------------------------------------------------------------------
// 1. The target and the look at the hand
// ---------------------------------------------------------------------

/// "Look at target opponent's hand": the question for the target offers the
/// two opponents and never the caster, and a caster named anyway is refused.
/// Then, at the choice, the controller is the one asked, the options are
/// exactly the player's hand, and he alone may inspect it (CR 722.4); before
/// Word resolves he could not, and the third seat never can.
#[test]
fn word_of_command_targets_an_opponent_and_shows_the_hand_at_the_choice() {
    let mut engine = table(&[mountain()], &[lightning_bolt(), index::JUGGERNAUT]);
    tap_swamps(&mut engine);
    cast_with_floating(&mut engine, USER, word_of_command());

    let (asked, mut offered) = opponents_offered(&engine);
    assert_eq!(asked, USER);
    offered.sort();
    assert_eq!(
        offered,
        vec![OTHER, THIRD],
        "an opponent, either of them, and never the caster"
    );
    let before = engine.fingerprint();
    let named_himself = match engine.pending() {
        Pending::ChoosePlayer { .. } => engine.apply(USER, PlayerAction::ChoosePlayer(USER)),
        _ => engine.apply(
            USER,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![USER],
            },
        ),
    };
    assert!(named_himself.is_err(), "the caster is no opponent");
    assert_eq!(engine.fingerprint(), before, "and nothing changed");
    assert!(
        !engine.may_inspect_private(USER, OTHER),
        "Word has not resolved: the hand is still private"
    );

    aim_word(&mut engine, OTHER);
    assert!(
        !engine.may_inspect_private(USER, OTHER),
        "a target is no control yet"
    );
    let options = reach_the_choice(&mut engine);

    let mut hand: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Hand(OTHER)).clone();
    let mut options_sorted = options.clone();
    hand.sort();
    options_sorted.sort();
    assert_eq!(options_sorted, hand, "the options are the player's hand");
    assert_eq!(hand.len(), 2);
    assert_eq!(engine.decision_actor(), Some(USER));
    // The look is the question itself: the engine asks the controller about
    // exactly these objects, and a view shows a seat what the seat is asked
    // about (`gamehost::view::zones::looking_at`). It asks nobody else, and
    // the control of the player (CR 722.4) only begins with the choice made.
    assert_eq!(
        engine.pending().asked(),
        Some(USER),
        "the hand is shown to the one who is asked"
    );
    assert!(
        !engine.may_inspect_private(THIRD, OTHER),
        "the third seat sees nothing"
    );
    assert!(
        !engine.may_inspect_private(USER, THIRD),
        "only the target is controlled"
    );
    assert!(!engine.may_inspect_private(OTHER, THIRD));

    // The choice is of a card out of *that* hand: a card of the third
    // seat's is no answer.
    let foreign = in_hand(&engine, THIRD, sol_ring()).unwrap();
    let before = engine.fingerprint();
    assert!(
        engine
            .apply(
                USER,
                PlayerAction::ChooseObjects {
                    objects: vec![foreign],
                },
            )
            .is_err()
    );
    assert_eq!(engine.fingerprint(), before);
    assert!(
        engine
            .apply(
                OTHER,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .is_err(),
        "the commanded player does not answer the question"
    );

    // With the card chosen, control begins: from here the controller may
    // inspect the player (CR 722.4) and answers for them (CR 722.5); the
    // third seat is still shut out.
    let bolt = in_hand(&engine, OTHER, lightning_bolt()).unwrap();
    engine
        .apply(
            USER,
            PlayerAction::ChooseObjects {
                objects: vec![bolt],
            },
        )
        .unwrap();
    assert!(engine.may_inspect_private(USER, OTHER));
    assert!(!engine.may_inspect_private(THIRD, OTHER));
    assert!(!engine.may_inspect_private(USER, THIRD));
    assert_eq!(engine.decision_actor(), Some(USER));
}

// ---------------------------------------------------------------------
// 2. A payable card is played with the opponent's lands and pool
// ---------------------------------------------------------------------

/// Lightning Bolt (`{R}`) is paid with the opponent's Mountain, tapped by the
/// controller; the spell is the opponent's on the stack, its target is the
/// controller's choice, and none of the controller's own resources is spent
/// beyond Word's `{B}{B}` (CR 722.5, 722.5a).
#[test]
fn a_payable_card_is_cast_with_the_opponents_lands_and_belongs_to_its_owner() {
    let mut engine = table(&[mountain(), mountain()], &[lightning_bolt()]);
    let bolt = command(&mut engine, lightning_bolt());

    // The commanded cast asks the controller for the target, in the
    // commanded player's name.
    let Pending::ChooseTargets { player, .. } = engine.pending().clone() else {
        panic!("Bolt's target expected, got {:?}", engine.pending())
    };
    assert_eq!(player, OTHER, "the question is the commanded player's");
    assert_eq!(
        engine.decision_actor(),
        Some(USER),
        "the controller answers"
    );
    aim_word(&mut engine, THIRD);

    assert!(in_commanded_window(&engine), "the player pays for it");
    let mountains = untapped(&engine, mountain());
    assert_eq!(mountains.len(), 2);
    assert_eq!(
        {
            let mut offered = offered_mana_sources(&engine);
            offered.sort();
            offered
        },
        {
            let mut all = mountains.clone();
            all.sort();
            all
        },
        "only the player's own lands can be tapped"
    );
    for &source in &mountains {
        assert_eq!(engine.state().object(source).unwrap().controller, OTHER);
    }
    engine
        .apply(
            USER,
            PlayerAction::ActivateManaAbility {
                source: mountains[0],
            },
        )
        .unwrap();
    assert!(is_tapped(&engine, mountains[0]));
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        1,
        "the mana is the player's"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(engine.state().players[2].mana_pool.total(), 0);
    if in_commanded_window(&engine) {
        engine.apply(USER, PlayerAction::PassPriority).unwrap();
    }

    let on_stack_now = engine.state().object(bolt).unwrap();
    assert_eq!(on_stack_now.zone, Zone::Stack);
    assert_eq!(on_stack_now.owner, OTHER);
    assert_eq!(
        on_stack_now.controller, OTHER,
        "the spell is controlled by its owner, not by whoever controls the player"
    );
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);
    assert!(
        !is_tapped(&engine, mountains[1]),
        "one Mountain pays for a one-mana spell"
    );
    for swamp in all_on_battlefield(&engine, USER, swamp()) {
        assert!(
            is_tapped(&engine, swamp),
            "Word's own payment, nothing more"
        );
    }
    assert!(in_graveyard(&engine, USER, word_of_command()).is_some());

    settle(&mut engine);
    assert_eq!(
        engine.state().players[2].life,
        17,
        "the third seat is the controller's choice of target"
    );
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20);
    assert!(in_graveyard(&engine, OTHER, lightning_bolt()).is_some());
    assert_eq!(engine.state().object(bolt).unwrap().owner, OTHER);
}

/// The player's pool pays too: red mana already floating in it when Word
/// resolves casts Lightning Bolt with no land tapped (CR 722.5a "mana").
#[test]
fn mana_already_in_the_players_pool_pays_for_the_card() {
    let mut engine = table(&[mountain(), mountain()], &[lightning_bolt()]);
    reach_their_main_phase(&mut engine, OTHER);
    let mountains = all_on_battlefield(&engine, OTHER, mountain());
    engine
        .apply(
            OTHER,
            PlayerAction::ActivateManaAbility {
                source: mountains[0],
            },
        )
        .unwrap();
    assert_eq!(engine.state().players[1].mana_pool.total(), 1);
    // Seat order after theirs is the third seat, then USER: the passes
    // hand priority round without ending the step, so the red floats on.
    engine.apply(OTHER, PlayerAction::PassPriority).unwrap();
    priority(&mut engine, USER);
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        1,
        "the red is still floating"
    );

    let bolt = command(&mut engine, lightning_bolt());
    let Pending::ChooseTargets { player, .. } = engine.pending().clone() else {
        panic!("Bolt's target expected, got {:?}", engine.pending())
    };
    assert_eq!(player, OTHER);
    aim_word(&mut engine, THIRD);
    if in_commanded_window(&engine) {
        // Nothing is owed that the pool does not cover: closing is allowed
        // and taps nothing.
        engine.apply(USER, PlayerAction::PassPriority).unwrap();
    }
    assert_eq!(engine.state().object(bolt).unwrap().zone, Zone::Stack);
    assert_eq!(engine.state().object(bolt).unwrap().controller, OTHER);
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "the floating red was spent"
    );
    assert!(!is_tapped(&engine, mountains[1]), "no further land tapped");
    settle(&mut engine);
    assert_eq!(engine.state().players[2].life, 17);
}

// ---------------------------------------------------------------------
// 3. An unpayable card is not played
// ---------------------------------------------------------------------

/// "Plays that card if able": a card the player's lands cannot pay for is not
/// played, whether the price is too large (Juggernaut, `{4}`, from one
/// Mountain) or of a colour they cannot make (Lightning Bolt from Plains and
/// Island). No payment is opened, nothing is tapped, no mana is made, the
/// card stays in hand, Word finishes and control ends.
#[test]
fn an_unpayable_card_is_not_played_and_nothing_is_tapped() {
    let cases: [(&str, &[CardIndex], CardIndex); 2] = [
        ("too dear", &[mountain()], index::JUGGERNAUT),
        ("wrong colours", &[plains(), island()], lightning_bolt()),
    ];
    for (why, board, card) in cases {
        let mut engine = table(board, &[card]);
        let chosen = command(&mut engine, card);
        assert!(
            !in_commanded_window(&engine),
            "{why}: no payment is opened for a price the lands cannot pay"
        );
        settle(&mut engine);
        assert_eq!(
            engine.state().object(chosen).unwrap().zone,
            Zone::Hand,
            "{why}: the card is not played"
        );
        for &card_index in board {
            for land in all_on_battlefield(&engine, OTHER, card_index) {
                assert!(!is_tapped(&engine, land), "{why}: nothing is tapped");
            }
        }
        assert_eq!(engine.state().players[1].mana_pool.total(), 0, "{why}");
        assert_eq!(engine.state().players[0].mana_pool.total(), 0, "{why}");
        assert!(
            in_graveyard(&engine, USER, word_of_command()).is_some(),
            "{why}: Word finished resolving"
        );
        assert!(
            !engine.may_inspect_private(USER, OTHER),
            "{why}: control ended with it"
        );
        assert_eq!(engine.state().players[0].life, 20, "{why}");
        assert_eq!(engine.state().players[1].life, 20, "{why}");
    }
}

// ---------------------------------------------------------------------
// 4. A land card
// ---------------------------------------------------------------------

/// Fixture for a land: `OTHER` has `lands` in hand and a Mountain out.
fn land_table(lands: &[CardIndex]) -> Engine<RegistryLookup> {
    table(&[mountain()], lands)
}

/// Moves to `OTHER`'s first main phase, where `USER` then holds priority.
#[track_caller]
fn to_their_main(engine: &mut Engine<RegistryLookup>) {
    reach_their_main_phase(engine, OTHER);
    priority(engine, USER);
}

/// CR 305.2a, 305.2b and 305.3 are the whole of the rule for a land the
/// effect tells a player to play: it is theirs only on their own turn and
/// only with a land drop left, and "ignore any part of an effect that
/// instructs a player to do so". On their turn with the drop unused the land
/// is played, by the owner, as a land and never as a spell.
#[test]
fn a_land_is_played_on_the_opponents_turn_with_a_land_drop_left() {
    let mut engine = land_table(&[forest(), plains()]);
    to_their_main(&mut engine);
    let land = command(&mut engine, forest());
    settle(&mut engine);
    let object = engine.state().object(land).unwrap();
    assert_eq!(object.zone, Zone::Battlefield, "played (CR 305.2a)");
    assert_eq!(object.controller, OTHER);
    assert_eq!(object.owner, OTHER);
    assert!(on_stack(&engine, forest()).is_none(), "a land is no spell");
    assert!(in_hand(&engine, OTHER, plains()).is_some());
    assert!(
        !engine.may_inspect_private(USER, OTHER),
        "control ended with Word"
    );
}

/// Not their turn (CR 305.3): on `USER`'s own turn the land stays in hand,
/// however much land drop the opponent has.
#[test]
fn a_land_is_not_played_when_it_is_not_the_opponents_turn() {
    let mut engine = land_table(&[forest()]);
    let land = command(&mut engine, forest());
    settle(&mut engine);
    assert_eq!(
        engine.state().object(land).unwrap().zone,
        Zone::Hand,
        "it is USER's turn, so OTHER cannot play a land (CR 305.3)"
    );
    assert_eq!(engine.state().object(land).unwrap().owner, OTHER);
    assert!(in_graveyard(&engine, USER, word_of_command()).is_some());
    assert!(!engine.may_inspect_private(USER, OTHER));
}

/// A land already played this turn takes the drop (CR 305.2b): the second
/// land stays in hand.
#[test]
fn a_land_is_not_played_when_the_opponent_has_no_land_drop_left() {
    let mut engine = land_table(&[forest(), plains()]);
    reach_their_main_phase(&mut engine, OTHER);
    let played = in_hand(&engine, OTHER, plains()).unwrap();
    engine
        .apply(OTHER, PlayerAction::PlayLand { card: played })
        .unwrap();
    priority(&mut engine, USER);
    let land = command(&mut engine, forest());
    settle(&mut engine);
    assert_eq!(
        engine.state().object(played).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state().object(land).unwrap().zone,
        Zone::Hand,
        "their land for the turn is spent (CR 305.2b)"
    );
    assert!(in_graveyard(&engine, USER, word_of_command()).is_some());
}

/// Two Words in a row on two lands: the first plays its land and uses the
/// drop (CR 305.2a counts "lands played during the resolution of spells and
/// abilities"), the second finds none left (CR 305.2b) and the land stays in
/// hand.
#[test]
fn a_land_played_by_word_uses_the_land_drop_of_the_next() {
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[word_of_command(), word_of_command()])
        .battlefield(1, &[mountain()])
        .hand(1, &[forest(), plains()])
        .battlefield(2, &[forest()])
        .hand(2, &[sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, OTHER);
    priority(&mut engine, USER);

    let first = command(&mut engine, forest());
    settle(&mut engine);
    assert_eq!(
        engine.state().object(first).unwrap().zone,
        Zone::Battlefield,
        "the first land is played"
    );
    priority(&mut engine, USER);
    let second = command(&mut engine, plains());
    settle(&mut engine);
    assert_eq!(
        engine.state().object(second).unwrap().zone,
        Zone::Hand,
        "the land drop is gone (CR 305.2b)"
    );
    assert!(!engine.may_inspect_private(USER, OTHER));
}

/// The main-phase and empty-stack conditions of CR 305.1 bind the special
/// action of a player with priority; an effect that instructs the play is
/// held only to 305.2 and 305.3. Word cast in the opponent's upkeep still
/// has them play the land.
#[test]
fn a_land_is_played_outside_a_main_phase_when_the_effect_instructs_it() {
    let mut engine = land_table(&[forest()]);
    pass_until(&mut engine, |e| {
        e.state().turn.active == OTHER
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == USER)
    });
    let land = command(&mut engine, forest());
    settle(&mut engine);
    assert_eq!(
        engine.state().object(land).unwrap().zone,
        Zone::Battlefield,
        "CR 305.2 and 305.3 are all that bind an effect's land play"
    );
}

// ---------------------------------------------------------------------
// 5. Mana from non-land sources
// ---------------------------------------------------------------------

/// "Only if they're from lands": with four Mountains and a Sol Ring the
/// payment of Juggernaut (`{4}`) is offered only the lands; tapping the Sol
/// Ring is refused and leaves nothing behind, and four Mountains pay.
#[test]
fn a_sol_ring_cannot_be_tapped_to_pay_for_the_card() {
    let mut engine = table(
        &[mountain(), mountain(), mountain(), mountain(), sol_ring()],
        &[index::JUGGERNAUT],
    );
    let juggernaut = command(&mut engine, index::JUGGERNAUT);
    assert!(in_commanded_window(&engine));
    let ring = on_battlefield(&engine, OTHER, sol_ring()).unwrap();
    let mountains = untapped(&engine, mountain());
    assert_eq!(mountains.len(), 4);
    let offered = offered_mana_sources(&engine);
    assert!(!offered.contains(&ring), "the ring is not on offer");
    for mountain in &mountains {
        assert!(offered.contains(mountain));
    }
    let before = engine.fingerprint();
    assert!(
        engine
            .apply(
                USER,
                PlayerAction::ActivateAbility {
                    source: ring,
                    ability_index: 0,
                },
            )
            .is_err(),
        "an artifact's mana is not allowed"
    );
    assert_eq!(engine.fingerprint(), before);
    assert!(!is_tapped(&engine, ring));
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);

    pay_with(&mut engine, &mountains);
    assert_eq!(
        engine.state().object(juggernaut).unwrap().zone,
        Zone::Stack,
        "the lands paid"
    );
    assert!(!is_tapped(&engine, ring), "the ring was never tapped");
    settle(&mut engine);
    assert!(on_battlefield(&engine, OTHER, index::JUGGERNAUT).is_some());
}

/// And where the lands alone fall short, the ring's two mana do not make up
/// the difference: one Mountain and two Sol Rings cannot pay `{4}`, so the
/// card is not played and the rings stay untapped.
#[test]
fn sol_rings_do_not_make_up_what_the_lands_lack() {
    let mut engine = table(&[mountain(), sol_ring(), sol_ring()], &[index::JUGGERNAUT]);
    let juggernaut = command(&mut engine, index::JUGGERNAUT);
    assert!(!in_commanded_window(&engine));
    settle(&mut engine);
    assert_eq!(engine.state().object(juggernaut).unwrap().zone, Zone::Hand);
    for ring in all_on_battlefield(&engine, OTHER, sol_ring()) {
        assert!(!is_tapped(&engine, ring));
    }
    assert_eq!(untapped(&engine, mountain()).len(), 1);
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);
}

// ---------------------------------------------------------------------
// 6. No surplus mana
// ---------------------------------------------------------------------

/// "Only if mana they produce is spent": Azorius Chancery makes `{W}{U}`, and
/// for Savannah Lions (`{W}`) the blue has nowhere to go, so tapping it is
/// refused before anything is done. Plains pays, and nothing floats after.
#[test]
fn an_overproducing_land_is_refused_and_no_mana_floats_afterwards() {
    let mut engine = table(
        &[plains(), index::AZORIUS_CHANCERY],
        &[index::SAVANNAH_LIONS],
    );
    // It enters tapped; their untap step is what makes it usable.
    to_their_main(&mut engine);
    let lions = command(&mut engine, index::SAVANNAH_LIONS);
    assert!(in_commanded_window(&engine));
    let chancery = on_battlefield(&engine, OTHER, index::AZORIUS_CHANCERY).unwrap();
    assert!(
        !is_tapped(&engine, chancery),
        "untapped, so a refusal below is the rule's and not the tap's"
    );
    let plains_land = on_battlefield(&engine, OTHER, plains()).unwrap();
    let before = engine.fingerprint();
    assert!(
        engine
            .apply(
                USER,
                PlayerAction::ActivateAbility {
                    source: chancery,
                    ability_index: 0,
                }
            )
            .is_err(),
        "{{W}}{{U}} would leave a blue nothing may spend"
    );
    assert_eq!(engine.fingerprint(), before, "nothing was tapped or made");
    assert!(!is_tapped(&engine, chancery));
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);

    pay_with(&mut engine, &[plains_land]);
    assert_eq!(engine.state().object(lions).unwrap().zone, Zone::Stack);
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "no mana is left floating"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(!is_tapped(&engine, chancery));
    settle(&mut engine);
    assert!(on_battlefield(&engine, OTHER, index::SAVANNAH_LIONS).is_some());
}

/// The positive control for the refusal above: where the price takes both
/// mana, the same Chancery is accepted. Crystalline Sliver costs `{W}{U}`.
#[test]
fn an_overproducing_land_is_accepted_when_the_card_uses_all_of_it() {
    let mut engine = table(&[index::AZORIUS_CHANCERY], &[index::CRYSTALLINE_SLIVER]);
    to_their_main(&mut engine);
    let sliver = command(&mut engine, index::CRYSTALLINE_SLIVER);
    assert!(in_commanded_window(&engine));
    let chancery = on_battlefield(&engine, OTHER, index::AZORIUS_CHANCERY).unwrap();
    assert!(
        !is_tapped(&engine, chancery),
        "untapped, so a refusal below is the rule's and not the tap's"
    );
    engine
        .apply(
            USER,
            PlayerAction::ActivateAbility {
                source: chancery,
                ability_index: 0,
            },
        )
        .unwrap();
    assert!(is_tapped(&engine, chancery));
    if in_commanded_window(&engine) {
        engine.apply(USER, PlayerAction::PassPriority).unwrap();
    }
    assert_eq!(engine.state().object(sliver).unwrap().zone, Zone::Stack);
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);
    settle(&mut engine);
    assert!(on_battlefield(&engine, OTHER, index::CRYSTALLINE_SLIVER).is_some());
}

// ---------------------------------------------------------------------
// 7. The controller makes the resolution's choices
// ---------------------------------------------------------------------

/// "If the chosen card is cast as a spell, you control the player while that
/// spell is resolving": Demonic Tutor's search is the controller's. The
/// question is the player's library and the player's (`player` is `OTHER`),
/// the controller answers it and the player does not, and the card goes to
/// the player's hand.
#[test]
fn the_controller_makes_the_commanded_spells_choices() {
    let mut engine = table(&[swamp(), swamp()], &[index::DEMONIC_TUTOR]);
    let tutor = command(&mut engine, index::DEMONIC_TUTOR);
    assert!(in_commanded_window(&engine));
    let swamps = untapped(&engine, swamp());
    pay_with(&mut engine, &swamps);
    assert_eq!(engine.state().object(tutor).unwrap().zone, Zone::Stack);
    assert!(in_graveyard(&engine, USER, word_of_command()).is_some());

    // Word is done and the Tutor is on the stack: the player's own turn to
    // respond, with no controller.
    priority(&mut engine, OTHER);
    assert_eq!(engine.decision_actor(), Some(OTHER));
    assert!(!engine.may_inspect_private(USER, OTHER));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, OTHER, "it is the player's search");
    assert_eq!(
        engine.decision_actor(),
        Some(USER),
        "and the controller's decision (CR 722.5)"
    );
    assert!(engine.may_inspect_private(USER, OTHER));
    assert!(!engine.may_inspect_private(THIRD, OTHER));
    for card in &options {
        assert_eq!(
            engine.state().object(*card).unwrap().zone,
            Zone::Library,
            "the player's library is what is searched"
        );
        assert_eq!(engine.state().object(*card).unwrap().owner, OTHER);
    }
    let pick = *options.last().expect("the library has cards");
    assert_ne!(pick, options[0], "the pick is not the first on offer");
    let before = engine.fingerprint();
    assert!(
        engine
            .apply(
                OTHER,
                PlayerAction::ChooseObjects {
                    objects: vec![pick]
                }
            )
            .is_err(),
        "the player may not answer while controlled"
    );
    assert_eq!(engine.fingerprint(), before);
    engine
        .apply(
            USER,
            PlayerAction::ChooseObjects {
                objects: vec![pick],
            },
        )
        .unwrap();
    settle(&mut engine);

    let picked = engine.state().object(pick).unwrap();
    assert_eq!(picked.zone, Zone::Hand);
    assert_eq!(picked.owner, OTHER);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(OTHER))
            .contains(&pick),
        "the search finds a card for the player it searched for"
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Hand(USER))
            .contains(&pick)
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}

// ---------------------------------------------------------------------
// 8. Control ends
// ---------------------------------------------------------------------

/// Control ends with the Word, or, when the card was cast, with the spell:
/// afterwards the controller cannot see the hand and the player's own
/// questions are theirs again. Checked at both ends: a Bolt in flight (the
/// Word has finished, the spell has not, so the player answers priority) and
/// the stack empty.
#[test]
fn control_ends_with_the_word_and_the_hand_is_private_again() {
    let mut engine = table(&[mountain()], &[lightning_bolt(), index::JUGGERNAUT]);
    command(&mut engine, lightning_bolt());
    aim_word(&mut engine, THIRD);
    let mountain = on_battlefield(&engine, OTHER, mountain()).unwrap();
    pay_with(&mut engine, &[mountain]);
    assert!(in_graveyard(&engine, USER, word_of_command()).is_some());

    // Word has finished; the Bolt waits on the stack for its resolution.
    priority(&mut engine, OTHER);
    assert_eq!(engine.decision_actor(), Some(OTHER));
    assert!(
        !engine.may_inspect_private(USER, OTHER),
        "the hand is private again once Word has resolved"
    );

    settle(&mut engine);
    assert!(in_graveyard(&engine, OTHER, lightning_bolt()).is_some());
    assert!(!engine.may_inspect_private(USER, OTHER));
    assert!(!engine.may_inspect_private(THIRD, OTHER));
    assert!(engine.may_inspect_private(OTHER, OTHER));
    priority(&mut engine, OTHER);
    assert_eq!(
        engine.decision_actor(),
        Some(OTHER),
        "their next decision is their own"
    );
    // And what they still hold is still theirs: the player may act on it.
    assert!(in_hand(&engine, OTHER, index::JUGGERNAUT).is_some());
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(legal.can_pass);
}
