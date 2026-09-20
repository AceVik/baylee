//! Every branch of `activate_card`, and the `Interaction` floor under it: what one tap on one card resolves to. There is no undo in the engine, so a spell arms on the first tap and sends on the second, cancel leaves nothing on the wire, and a card that is both a land and a spell refuses to guess which — while a land plays on the click and a mana ability stays one tap, because floating mana is the cheap mistake (CR 605.1). The last branch plays nothing at all: a tap on the top of a pile opens it, and a library never opens however often it is tapped (CR 401.2). Asserted on `Duel` and not on `Interaction`, because the claim is about *taps* — that a resolver's second call returns an action would pass just as well if the first call had already sent it.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn confirming_a_priority_choice_passes() {
    let i = Interaction::new(
        Pending::Priority {
            player: PlayerId::new(0),
            legal: Box::new(LegalActions {
                can_pass: true,
                lands: vec![obj(1)],
                castable: vec![],
                mana_abilities: vec![],
                abilities: vec![],
                suspendable: vec![],
            }),
        },
        PlayerId::new(0),
    );
    assert_eq!(i.confirm(), Some(PlayerAction::PassPriority));
    // And a click on the land plays it instead of passing.
    assert_eq!(
        i.play_card(obj(1)),
        Some(PlayerAction::PlayLand { card: obj(1) })
    );
}

/// A tap on the top card of a pile opens that pile.
///
/// This is the only way in to a graveyard now that the pile chips are
/// gone, so it needs a witness rather than a reading: `open_pile` was
/// wired before the chips were removed and nothing ever clicked it, which
/// is precisely the shape of defect this client has shipped before. The
/// door has to be proved from a *tap* — `activate_card`, the same
/// function the pointer calls — and not by calling `open_pile` directly,
/// or the test would pass with the last branch of `activate_card` gone.
#[test]
fn a_tap_on_a_pile_opens_it() {
    use baylee_client_core::test_support::{ViewBuilder, printed};

    let top = obj(7);
    let view = ViewBuilder::new(2)
        .with_graveyard(0, vec![printed(7, 0, "Llanowar Elves", 1)])
        .build();
    let mut duel = crate::Duel::default();
    duel.receive_view(view);
    crate::rebuild_board(&mut duel);
    assert!(!duel.browser.is_open(), "nothing has been tapped yet");

    activate_card(&mut duel, top);
    assert!(duel.browser.is_open(), "the graveyard did not open");
    assert_eq!(
        duel.browser.ticked().iter().copied().collect::<Vec<_>>(),
        vec![baylee_client_core::BrowseZone::Graveyard(PlayerId::new(0))],
        "it opened on somebody else's pile"
    );
}

/// And a library never opens, however often it is tapped: nobody may look
/// through one, their own included (CR 401.2), so the pile beside the mat
/// is inert rather than merely empty.
///
/// The pile is **manufactured**, because a view cannot produce one: a
/// library is face down to everybody, so `ZonePile::top` is always `None`
/// there and no tap can ever name its card. It is built anyway because a
/// test that tapped an object on no pile at all would pass with every
/// guard in `open_pile` deleted, and would then be claiming CR 401.2 while
/// holding nothing. Two independent readings refuse it — `is_browsable`
/// and `BrowseZone::of_pile` — and each has its own witness in
/// `baylee-client-core`; what is asserted here is the outcome a player
/// sees.
#[test]
fn a_tap_on_a_library_opens_nothing() {
    use baylee_client_core::layout::PileKind;
    use baylee_client_core::test_support::ViewBuilder;

    let top = obj(7);
    let view = ViewBuilder::new(2).build();
    let mut duel = crate::Duel::default();
    duel.receive_view(view);
    crate::rebuild_board(&mut duel);
    {
        let board = duel.board.as_mut().expect("the view built a board");
        let pile = board.pods[0]
            .piles
            .iter_mut()
            .find(|pile| pile.kind == PileKind::Library)
            .expect("every seat has a library");
        pile.count = 60;
        pile.top = Some(top);
    }

    activate_card(&mut duel, top);
    assert!(!duel.browser.is_open());
}

#[test]
fn a_land_plays_on_the_click() {
    let land = obj(3);
    let mut duel = window_with(vec![land], vec![]);

    activate_card(&mut duel, land);
    assert_eq!(
        duel.outbox(),
        [PlayerAction::PlayLand { card: land }],
        "the land did not play on the click"
    );
    assert!(duel.armed.is_none(), "a land asked for a confirmation");
}

/// There is no undo in the engine and there should not be one, so the
/// client owes a player the chance to take a tap back before it becomes a
/// game action. Tested at this level and not on `Interaction`, because
/// what is being claimed is about *taps*: an assertion that the second
/// call to a resolver returns an action would pass just as well if the
/// first one had already sent it.
#[test]
fn a_spell_still_arms_and_a_second_tap_sends_it() {
    let spell = obj(4);
    let mut duel = window_with(vec![], vec![spell]);

    activate_card(&mut duel, spell);
    assert!(
        duel.outbox().is_empty(),
        "the first tap put a spell on the wire"
    );
    assert_eq!(
        duel.armed,
        Some(crate::Armed {
            object: spell,
            deed: crate::Deed::Play
        })
    );

    activate_card(&mut duel, spell);
    assert_eq!(duel.outbox(), [PlayerAction::CastSpell { card: spell }]);
    assert!(duel.armed.is_none(), "firing left the deed armed");
}

/// The exception to the exception. A modal double-faced card with a spell
/// front and a land back is in *both* lists, and `play_card` checks lands
/// first — so one-clicking it would resolve it to "play as land" every
/// time and the front face would be unreachable by mouse.
#[test]
fn a_card_that_is_both_a_land_and_a_spell_does_not_one_click() {
    let mdfc = obj(5);
    let mut duel = window_with(vec![mdfc], vec![mdfc]);

    activate_card(&mut duel, mdfc);
    assert!(
        duel.outbox().is_empty(),
        "a card with two ways to play it fired one of them on the click"
    );
    assert!(duel.armed.is_some());
}

/// Cancel is the whole point of arming: it has to leave nothing behind.
#[test]
fn cancel_disarms_with_nothing_on_the_wire() {
    use crate::keys::Fired;
    use baylee_client_core::prefs::{Action, Chord, Keymap};
    use bevy::prelude::KeyCode;

    // A spell, because a land no longer arms at all.
    let spell = obj(4);
    let mut duel = window_with(vec![], vec![spell]);

    activate_card(&mut duel, spell);
    assert!(duel.armed.is_some());

    let mut keymap = Keymap::standard();
    keymap.bind(Action::Cancel, vec![Chord::key("Escape")]);
    let mut keys = bevy::input::ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::Escape);
    assert!(armed_keys(Fired::of(&keys, &keymap), &mut duel));
    assert!(duel.armed.is_none(), "cancel left the deed armed");
    assert!(duel.outbox().is_empty(), "cancel sent something");
}

/// The exception, and the reason it is one: floating mana is the cheap
/// mistake in this game, so tapping a land stays a single tap.
#[test]
fn a_mana_ability_still_goes_through_on_one_tap() {
    use crate::host::DuelHost;
    let (mut duel, mut host) = duel_in_main_phase();
    let land = duel
        .interaction
        .as_ref()
        .and_then(Interaction::legal_actions)
        .and_then(|l| l.lands.first().copied())
        .expect("the window offers a land");

    // Put the land in play first — it is the only mana source this deck
    // has, and a land in hand makes no mana. One call: a land plays on
    // the click now, and a second would send `PlayLand` twice.
    activate_card(&mut duel, land);
    // `outbox` is private to `Duel` but declared in the crate root, so a
    // child module may drain it — which is what `flush_outbox` does in
    // the running client.
    for action in std::mem::take(&mut duel.outbox) {
        host.submit(action);
    }
    for message in host.poll() {
        match message {
            crate::host::HostMessage::View(v) => duel.view = Some(*v),
            crate::host::HostMessage::Choice(p) => {
                duel.interaction = Some(Interaction::new(*p, PlayerId::new(0)));
            }
            _ => {}
        }
    }
    let source = duel
        .interaction
        .as_ref()
        .and_then(Interaction::legal_actions)
        .and_then(|l| l.mana_abilities.first().copied())
        .expect("the land that was just played can be tapped");

    activate_card(&mut duel, source);
    assert!(
        duel.armed.is_none(),
        "tapping for mana asked for a confirmation"
    );
    assert_eq!(
        duel.outbox(),
        [PlayerAction::ActivateManaAbility { source }]
    );
}

use crate::host::{DuelHost, HostMessage, LocalHost};
use baylee_core::ids::ObjectId;

/// Carry one frame between a real host and the client, in `lib.rs`'s order.
///
/// Poll, apply, advance the run, flush. A test that flushed before it
/// advanced would be testing an order no running client uses.
///
/// The last line is the one that is easy to leave out, and leaving it out is
/// what makes a harness disagree with the client it is standing in for:
/// `flush_outbox` does one thing more than [`crate::Duel::take_outbox`] does
/// — it drops the interaction, because the answer has been sent and the next
/// choice replaces it. Until that choice arrives the seat has no legal
/// actions, so every card in hand is neither `playable` nor offered, and a
/// pump that skipped this would never see that window at all.
fn pump(host: &mut LocalHost, duel: &mut crate::Duel) {
    for m in host.poll() {
        match m {
            HostMessage::Static(s) => duel.statics = Some(*s),
            HostMessage::View(v) => {
                duel.receive_view(*v);
                crate::rebuild_board(duel);
            }
            HostMessage::Choice(p) => duel.receive_choice(*p),
            HostMessage::Failed(e) => panic!("the host refused: {e}"),
        }
    }
    crate::advance_mana_run(duel);
    let sent = duel.take_outbox();
    let answered = !sent.is_empty();
    for action in sent {
        host.submit(action);
    }
    if answered {
        duel.interaction = None;
    }
}

/// A real duel played to the window #112 reports, and the instant in hand.
///
/// It is the half a hand-built view cannot honestly stand in for.
/// [`crate::reachable`] reads projected types, the printed cost and the
/// seat's own mana sources, and `host::tests::duel_preset` already says why
/// that matters: *a view assembled by a test is a view that agrees with
/// whatever the test expected of it.* So this runs a real `LocalHost`
/// against the house AI and pumps its messages the way `lib.rs` pumps them.
///
/// The position is the one reported: an opponent's spell on the stack, one
/// untapped Plains, and an instant in hand.
fn the_reported_window() -> (LocalHost, crate::Duel, ObjectId) {
    use baylee_core::ids::PrintRef;
    use baylee_core::preset::{
        AIProfile, DeckEntry, Finish, FormatId, GamePreset, HouseRules, PrintInfo,
        SeatCapabilities, SeatController, SeatSpec,
    };

    let entry = |name: &str| DeckEntry {
        card: baylee_cards::decks::by_name(name).unwrap_or_else(|| panic!("`{name}` in the pool")),
        print: PrintRef::new(0),
    };
    let filler: Vec<DeckEntry> = (0..40).map(|_| entry("Island")).collect();
    let seat = |ai: bool, hand: Vec<DeckEntry>, field: Vec<DeckEntry>| SeatSpec {
        controller: if ai {
            SeatController::Ai(AIProfile::default())
        } else {
            SeatController::Open
        },
        capabilities: SeatCapabilities::default(),
        deck: filler.clone(),
        sideboard: vec![],
        commanders: vec![],
        starting_life: None,
        starting_hand: Some(hand),
        starting_battlefield: field,
        emblems: vec![],
        team: None,
    };
    // The AI seat holds exactly one card and the lands to pay for it, so the
    // spell that reaches the stack is not a bet on what a heuristic felt like
    // doing. If this ever stops arriving the test says so out loud rather
    // than passing over an empty stack.
    let preset = GamePreset {
        format: FormatId::Freeform,
        seed: 11,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![
            seat(
                false,
                vec![entry("Swords to Plowshares")],
                vec![entry("Plains")],
            ),
            seat(
                true,
                vec![entry("Eerie Interlude")],
                vec![
                    entry("Baleful Strix"),
                    entry("Plains"),
                    entry("Plains"),
                    entry("Plains"),
                ],
            ),
        ],
    };

    let mut host =
        LocalHost::new(&preset, PlayerId::new(0), &["You", "House"]).expect("the duel starts");
    let mut duel = crate::Duel::default();

    // Play on until this seat holds priority over something on the stack.
    let mut spell = None;
    for _ in 0..400 {
        pump(&mut host, &mut duel);
        let Some(pending) = duel.interaction.as_ref().map(|i| i.pending().clone()) else {
            continue;
        };
        let stacked = duel.view.as_ref().is_some_and(|v| !v.stack.is_empty());
        match &pending {
            Pending::Priority { player, .. } if *player == PlayerId::new(0) && stacked => {
                spell = duel
                    .view
                    .as_ref()
                    .and_then(|v| v.hand.first().map(|h| h.id));
                break;
            }
            Pending::Mulligan { player, .. } if *player == PlayerId::new(0) => {
                host.submit(PlayerAction::MulliganKeep);
            }
            Pending::Priority { player, .. } if *player == PlayerId::new(0) => {
                host.submit(PlayerAction::PassPriority);
            }
            _ => {}
        }
    }
    let spell = spell.expect(
        "the opponent never put a spell on the stack, so the window this test \
         is about never opened — re-arrange the position rather than relaxing \
         the assertions below",
    );
    (host, duel, spell)
}

/// The end of the gesture, which is the half a tap-level assertion misses.
///
/// A run that arms and never sends is precisely the fault that was reported,
/// and it passes every assertion that stops at `armed`. Sending is also not
/// landing: on the frame the run ends the seat is still holding priority
/// with its mana floating, so a test that read the pending here would be
/// reading its own last frame. The engine gets its answer first, and the
/// proof that it took the cast is that it turns round and asks this seat for
/// the spell's target.
fn the_gesture_lands_on_a_target(host: &mut LocalHost, duel: &mut crate::Duel) {
    for _ in 0..40 {
        pump(host, duel);
        if duel.mana_run.is_none() {
            break;
        }
    }
    assert!(
        duel.mana_run.is_none(),
        "the run never finished: {:?}",
        duel.last_error
    );
    assert_eq!(duel.last_error, None, "the gesture ended on an error");

    for _ in 0..20 {
        pump(host, duel);
        if matches!(
            duel.interaction.as_ref().map(Interaction::pending),
            Some(Pending::ChooseTargets { .. })
        ) {
            break;
        }
    }
    assert!(
        matches!(
            duel.interaction.as_ref().map(Interaction::pending),
            Some(Pending::ChooseTargets { options, .. }) if !options.is_empty()
        ),
        "after the gesture the engine is at {:?}",
        duel.interaction.as_ref().map(Interaction::pending)
    );
}

/// The indigo half of the hand, clicked the way a player clicks it.
///
/// Every other test in this file walks the **gold** path, and not by choice:
/// `window_with` carries no view at all, and [`crate::reachable`] returns an
/// empty set without one. So "this client will tap your lands for you" — the
/// other half of the hand, and two of the four branches of `activate_card` —
/// had never been reached by a tap.
#[test]
fn the_indigo_half_of_the_hand_casts_on_two_taps() {
    let (mut host, mut duel, spell) = the_reported_window();

    // Lit, and lit by whichever authority is supposed to be lighting it.
    let card = duel
        .board
        .as_ref()
        .and_then(|b| b.hand.iter().find(|h| h.id == spell))
        .expect("the card is in the rendered hand");
    assert!(
        !card.playable && card.reachable,
        "the engine has not offered this cast and the client has: {card:?}"
    );

    activate_card(&mut duel, spell);
    assert!(
        matches!(
            duel.armed,
            Some(crate::Armed {
                deed: crate::Deed::Run {
                    then: crate::RunEnd::Cast,
                    ..
                },
                ..
            })
        ),
        "the first tap armed {:?}",
        duel.armed
    );
    assert!(duel.outbox().is_empty(), "the first tap put it on the wire");

    activate_card(&mut duel, spell);
    assert!(
        duel.mana_run.is_some(),
        "the second tap started no run: {:?}",
        duel.last_error
    );

    // `pump` carries each step of the run to the engine in the order the run
    // emits them — the land first, the spell only once its mana is floating.
    the_gesture_lands_on_a_target(&mut host, &mut duel);
}

/// The same window with the mana **already in the pool**, which is the
/// position actually reported.
///
/// The owner's words were "ich habe ein weißes Mana im Pool". That is the
/// *gold* path — the engine has verified the cost and offers the cast
/// outright — and it is a different branch of `activate_card` from the
/// indigo one: `Deed::Play` and no run at all. `a_spell_still_arms_and_a_
/// second_tap_sends_it` covers that branch over a stub, and the first #112
/// probe covered it over an `Interaction`; neither had a board model, an
/// opponent's spell on the stack, or a real projection underneath it.
#[test]
fn the_gold_half_casts_with_the_mana_already_in_the_pool() {
    let (mut host, mut duel, spell) = the_reported_window();

    // Tapping the land is also the likeliest way a white mana got into a
    // player's pool in this window, so this arm gets there the way he would
    // have.
    let source = duel
        .interaction
        .as_ref()
        .and_then(Interaction::legal_actions)
        .and_then(|l| l.mana_abilities.first().copied())
        .expect("an untapped land to tap");
    activate_card(&mut duel, source);
    for _ in 0..20 {
        pump(&mut host, &mut duel);
        if duel
            .interaction
            .as_ref()
            .and_then(Interaction::legal_actions)
            .is_some_and(|l| l.castable.contains(&spell))
        {
            break;
        }
    }

    // **Tapping for mana does not pass priority**, which is worth asserting
    // rather than inferring. A mana ability does not use the stack (CR
    // 605.3b) and the player keeps priority, so the opponent's spell is still
    // sitting on it — and if that were ever untrue, this whole window would
    // close the instant a player reached for their own mana, which is
    // precisely the sequence the owner described.
    assert!(
        duel.view.as_ref().is_some_and(|v| !v.stack.is_empty()),
        "the opponent's spell left the stack while this seat tapped a land"
    );
    assert!(
        matches!(
            duel.interaction.as_ref().map(Interaction::pending),
            Some(Pending::Priority { player, .. }) if *player == PlayerId::new(0)
        ),
        "this seat stopped holding priority over its own mana ability: {:?}",
        duel.interaction.as_ref().map(Interaction::pending)
    );

    let card = duel
        .board
        .as_ref()
        .and_then(|b| b.hand.iter().find(|h| h.id == spell))
        .expect("the card is in the rendered hand");
    assert!(
        card.playable && !card.reachable,
        "with the mana floating the engine itself offers the cast, so it is \
         gold and not this client's own offer: {card:?}"
    );

    activate_card(&mut duel, spell);
    assert!(
        matches!(
            duel.armed,
            Some(crate::Armed {
                deed: crate::Deed::Play,
                ..
            })
        ),
        "nothing needs tapping, so the first tap arms a plain play: {:?}",
        duel.armed
    );
    assert!(duel.outbox().is_empty(), "the first tap put it on the wire");

    activate_card(&mut duel, spell);
    assert!(
        duel.mana_run.is_none(),
        "there was nothing left to tap, so no run should have started"
    );
    assert_eq!(
        duel.outbox(),
        [PlayerAction::CastSpell { card: spell }],
        "the second tap did not send the cast: {:?}",
        duel.last_error
    );

    the_gesture_lands_on_a_target(&mut host, &mut duel);
}

/// The reported position, one resolution later: the card goes dark and the
/// click is swallowed without a word.
///
/// This is #112 reproduced. Eerie Interlude *exiles* the creatures it targets
/// (CR 702.x is not the point; the card is), so a player who lets it resolve
/// while holding removal has no legal target left — and the engine's
/// `castable` correctly stops offering the cast. The client is right about
/// the rules and silent about them:
///
/// - `playable` is false, because the engine will not accept the cast.
/// - `reachable` is false, because the only land has already been tapped.
/// - so the card is drawn in neither gold nor indigo, which is **dark**.
/// - and the tap arms nothing, sends nothing, and sets no `last_error`.
///
/// That is all three of the owner's answers at once — "dark / greyed out like
/// unplayable", "the `{W}` came from tapping a land", "clicking did nothing at
/// all" — with no defect anywhere in the input path. What is wrong is that the
/// same dark card and the same swallowed tap mean "you cannot afford it",
/// "it is not the right time" and "there is nothing left to target", and the
/// client distinguishes none of them.
///
/// **The last assertion is the defect, pinned.** `last_error` staying `None`
/// is the silence; the day this client explains a refused tap, this test goes
/// red, and that is the outcome it exists to produce.
#[test]
fn a_spell_whose_only_target_was_exiled_goes_dark_and_says_nothing() {
    let (mut host, mut duel, spell) = the_reported_window();
    let on_the_battlefield = |duel: &crate::Duel, what: &str| {
        duel.view
            .as_ref()
            .is_some_and(|v| v.battlefield.iter().any(|o| o.name == what))
    };
    assert!(
        on_the_battlefield(&duel, "Baleful Strix"),
        "the creature the removal is for has to be there before it is not"
    );

    // Get the mana up, the way he did.
    let source = duel
        .interaction
        .as_ref()
        .and_then(Interaction::legal_actions)
        .and_then(|l| l.mana_abilities.first().copied())
        .expect("an untapped land to tap");
    activate_card(&mut duel, source);
    for _ in 0..20 {
        pump(&mut host, &mut duel);
        if duel
            .interaction
            .as_ref()
            .and_then(Interaction::legal_actions)
            .is_some_and(|l| l.castable.contains(&spell))
        {
            break;
        }
    }

    // Then let the opponent's spell resolve. Passing by hand rather than
    // through the automation on purpose: the client's own auto-pass could not
    // have done this — it is guarded twice over, by `offering` and by
    // `opposing_stack` — so a test that leant on it would be asserting
    // something this position cannot reach.
    for _ in 0..60 {
        if let Some(PlayerAction::PassPriority) =
            duel.interaction.as_ref().and_then(Interaction::confirm)
        {
            duel.submit(PlayerAction::PassPriority);
        }
        pump(&mut host, &mut duel);
        if duel.view.as_ref().is_some_and(|v| v.stack.is_empty())
            && duel.interaction.as_ref().is_some_and(Interaction::is_mine)
        {
            break;
        }
    }
    assert!(
        !on_the_battlefield(&duel, "Baleful Strix"),
        "the spell never resolved, so this is not the position under test"
    );

    let card = duel
        .board
        .as_ref()
        .and_then(|b| b.hand.iter().find(|h| h.id == spell))
        .expect("the card is still in the rendered hand");
    assert!(
        !card.playable && !card.reachable,
        "neither authority lights it, which is what dark means: {card:?}"
    );

    // And the tap. Both halves matter: nothing happens, and nothing is said.
    activate_card(&mut duel, spell);
    assert!(
        duel.armed.is_none(),
        "the tap armed {:?} on a card the engine will not accept",
        duel.armed
    );
    assert!(
        duel.outbox().is_empty(),
        "the tap put {:?} on the wire",
        duel.outbox()
    );
    assert_eq!(
        duel.last_error, None,
        "this is the finding and not the fixture: the client refuses the tap \
         and tells the player nothing, so there is no refusal to read. When \
         it learns to say why, this assertion is the one that fails."
    );
}

/// The client stops offering to tap lands for a spell with nothing to point
/// at — and still offers when there is something (#139).
///
/// Both halves in one test on purpose. The claim is a *negative*, and a
/// negative about a card in hand has a dozen ways to be true by accident: the
/// mana might not reach, the window might be shut, the card might not be read
/// at all. The second board is the premise made assertable — same seat, same
/// two Plains, same card, one creature added — so the first board's silence
/// can only be about the target.
///
/// What it costs when it is wrong is why it is a P0: the offer is taken by a
/// click, the click arms a mana run, the run taps the lands, and *then* the
/// engine refuses the cast. The turn's mana is gone and the card is still in
/// hand.
#[test]
fn a_spell_with_nothing_to_point_at_is_not_offered_the_lands() {
    let bare = the_table_with(&[]);
    let swords = bare
        .1
        .view
        .as_ref()
        .and_then(|v| v.hand.iter().find(|c| c.name == "Swords to Plowshares"))
        .map(|c| c.id)
        .expect("the seat is holding it");
    assert!(
        !bare.1.reachable.contains(&swords),
        "there is no creature on the table, so tapping for it spends the turn \
         on a cast the engine will refuse"
    );

    let (_host, duel) = the_table_with(&["Llanowar Elves"]);
    let swords = duel
        .view
        .as_ref()
        .and_then(|v| v.hand.iter().find(|c| c.name == "Swords to Plowshares"))
        .map(|c| c.id)
        .expect("the seat is holding it");
    assert!(
        duel.reachable.contains(&swords),
        "with a creature to point at, the two Plains do reach {{W}} — which \
         is what makes the first half of this test a claim about targets and \
         not about mana"
    );
}

/// Seat 0 in its own main phase, holding Swords to Plowshares over two
/// Plains, with `creatures` on the opposing battlefield.
fn the_table_with(creatures: &[&str]) -> (LocalHost, crate::Duel) {
    use baylee_core::ids::PrintRef;
    use baylee_core::preset::{
        AIProfile, DeckEntry, Finish, FormatId, GamePreset, HouseRules, PrintInfo,
        SeatCapabilities, SeatController, SeatSpec,
    };

    let entry = |name: &str| DeckEntry {
        card: baylee_cards::decks::by_name(name).unwrap_or_else(|| panic!("`{name}` in the pool")),
        print: PrintRef::new(0),
    };
    // Islands, so neither seat can ever draw a creature and make the bare
    // board stop being bare halfway through the walk.
    let filler: Vec<DeckEntry> = (0..40).map(|_| entry("Island")).collect();
    let seat = |ai: bool, hand: Vec<DeckEntry>, field: Vec<DeckEntry>| SeatSpec {
        controller: if ai {
            SeatController::Ai(AIProfile::default())
        } else {
            SeatController::Open
        },
        capabilities: SeatCapabilities::default(),
        deck: filler.clone(),
        sideboard: vec![],
        commanders: vec![],
        starting_life: None,
        starting_hand: Some(hand),
        starting_battlefield: field,
        emblems: vec![],
        team: None,
    };
    let preset = GamePreset {
        format: FormatId::Freeform,
        seed: 11,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![
            seat(
                false,
                vec![entry("Swords to Plowshares")],
                vec![entry("Plains"), entry("Plains")],
            ),
            seat(
                true,
                vec![],
                creatures.iter().map(|name| entry(name)).collect(),
            ),
        ],
    };

    let mut host =
        LocalHost::new(&preset, PlayerId::new(0), &["You", "House"]).expect("the duel starts");
    let mut duel = crate::Duel::default();
    for _ in 0..400 {
        pump(&mut host, &mut duel);
        let Some(pending) = duel.interaction.as_ref().map(|i| i.pending().clone()) else {
            continue;
        };
        let open = duel.view.as_ref().is_some_and(|v| {
            v.active == v.seat
                && v.stack.is_empty()
                && matches!(
                    v.phase,
                    baylee_view::Phase::FirstMain | baylee_view::Phase::SecondMain
                )
        });
        match &pending {
            Pending::Priority { player, .. } if *player == PlayerId::new(0) && open => break,
            Pending::Mulligan { player, .. } if *player == PlayerId::new(0) => {
                host.submit(PlayerAction::MulliganKeep);
            }
            Pending::Priority { player, .. } if *player == PlayerId::new(0) => {
                host.submit(PlayerAction::PassPriority);
            }
            _ => {}
        }
    }
    (host, duel)
}
