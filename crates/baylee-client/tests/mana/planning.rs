//! Which lands the planner taps, and when it refuses to plan at all.
//!
//! The question here is arithmetic over the board: can these permanents pay
//! for this spell, and which of them does the client tap to do it. A land
//! that asks a question back is part of the plan too, because the plan has
//! to stop and wait for the answer.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn the_client_taps_the_lands_a_spell_needs_and_then_casts_it() {
    let mut table = Table::open();
    table.walk_to_main();

    let spell = table
        .view()
        .hand
        .iter()
        .find(|c| c.name == "Great Divide Guide")
        .expect("the creature is in the opening hand")
        .id;

    // The premise: the engine has *not* offered it, because nothing is
    // floating. Without this line the rest of the test would pass for the
    // wrong reason.
    assert!(
        !table.legal().castable.contains(&spell),
        "the engine should not offer a spell whose mana is not floating"
    );

    // Three, not six. A Forest is offered twice by `LegalActions` — once as
    // the CR 305.6 shortcut and once as the `{T}: Add {G}` printed on the card
    // — and it can still only be tapped once.
    let sources = manasources::sources(table.view(), table.legal());
    assert_eq!(sources.len(), 3, "three untapped Forests");

    let cost = manasources::hand_cost(
        table
            .view()
            .hand
            .iter()
            .find(|c| c.id == spell)
            .expect("still in hand"),
    )
    .expect("a printed cost");
    let pool = table
        .view()
        .seat(PlayerId::new(0))
        .expect("own seat")
        .mana_pool;
    let plan = manaplan::plan(&cost, &pool, &sources).expect("{1}{G} out of three Forests");
    assert_eq!(plan.taps(), 2, "two Forests, not three");

    // Exactly what the client's own run does: tap, then cast, re-checking the
    // engine's offer at every step.
    for step in &plan.steps {
        assert!(
            table.legal().mana_abilities.contains(&step.source),
            "the engine still offers the land the plan picked"
        );
        assert_eq!(step.color, None, "a Forest is never asked which colour");
        table.submit(PlayerAction::ActivateManaAbility {
            source: step.source,
        });
    }

    assert!(
        table.legal().castable.contains(&spell),
        "with the mana floating the engine offers the spell"
    );
    table.submit(PlayerAction::CastSpell { card: spell });

    // Cast, resolved through both seats passing, and on the battlefield.
    for _ in 0..40 {
        if table
            .view()
            .battlefield
            .iter()
            .any(|o| o.name == "Great Divide Guide")
        {
            return;
        }
        match table.pending.clone() {
            Some(Pending::Priority { player, .. }) if player == PlayerId::new(0) => {
                table.submit(PlayerAction::PassPriority);
            }
            _ => break,
        }
    }
    panic!("the spell never reached the battlefield");
}

/// The other half of the claim: a spell nothing on the table can pay for is
/// not offered a plan either. A client that says yes here would tap two lands
/// and then stop, which is worse than saying no.
#[test]
fn a_spell_the_lands_cannot_pay_for_gets_no_plan() {
    let mut table = Table::open();
    table.walk_to_main();

    let sources = manasources::sources(table.view(), table.legal());
    let pool = table
        .view()
        .seat(PlayerId::new(0))
        .expect("own seat")
        .mana_pool;

    // Three Forests, and a cost with a blue pip in it.
    let cost = baylee_core::mana::ManaCost::try_parse("{1}{U}").expect("a valid cost");
    assert!(manaplan::plan(&cost, &pool, &sources).is_none());

    // …and one that is simply too expensive.
    let cost = baylee_core::mana::ManaCost::try_parse("{7}").expect("a valid cost");
    assert!(manaplan::plan(&cost, &pool, &sources).is_none());
}

/// The fetchland bug, end to end.
///
/// Reported from a live game: "Fetchland Effekt fügt irgendwas in den Manapool
/// statt mich ein passendes Land aus der Bibliothek auswählen zu lassen."
///
/// The card was never the fault. `Interaction::activate` routed on the ability
/// index's *numeric value* — a permanent named in `mana_abilities` plus an
/// index of 0 meant "mana ability", whatever the engine had offered there. The
/// Lantern grants every land a mana ability, and Bloodstained Mire's real
/// ability sits at index 0. Both cards are in the starter deck the lobby
/// posts, which is why this was met in the first game.
///
/// Driven through `activate_card` rather than through `Interaction`, because
/// what is claimed is about a *click*: an assertion on the resolver would pass
/// just as well if no click could reach it.
#[test]
fn a_fetchland_under_a_chromatic_lantern_still_searches() {
    use baylee_client::Duel;
    use baylee_client::input::{ability_menu_keys, activate_card, armed_keys};
    use baylee_client::keys::Fired;
    use baylee_client_core::interaction::Interaction;
    use baylee_client_core::prefs::Action;

    let mut preset = preset();
    preset.seats[0].starting_battlefield.push(entry(MIRE));
    preset.seats[0].starting_battlefield.push(entry(LANTERN));
    let mut table = Table::open_with(&preset);
    table.walk_to_main();

    let mire = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Bloodstained Mire")
        .expect("the fetchland is on the table")
        .id;

    // The precondition, so the test cannot pass by the grant never arriving:
    // the engine must be naming the Mire in *both* lists at once.
    let legal = match table.pending.as_ref().expect("priority") {
        baylee_engine::choice::Pending::Priority { legal, .. } => legal.clone(),
        other => panic!("not a priority window: {other:?}"),
    };
    assert!(
        legal.mana_abilities.contains(&mire),
        "the Lantern did not grant the fetchland a mana ability"
    );
    assert!(
        legal.abilities.contains(&(mire, 0)),
        "the fetchland's own ability is not at index 0"
    );

    let mut duel = Duel::default();
    duel.view = Some(table.view().clone());
    duel.interaction = Some(Interaction::new(
        table.pending.clone().expect("priority"),
        PlayerId::new(0),
    ));

    // The list is where the bug was visible: `Interaction::activate` turned
    // index 0 into a mana ability, so the search was not in it at all.
    let options = baylee_client::abilities::options(
        baylee_client_core::Lang::En,
        duel.view.as_ref().expect("a view"),
        duel.interaction.as_ref().expect("a choice"),
        mire,
    );
    let search = options
        .iter()
        .position(|o| {
            o.action
                == PlayerAction::ActivateAbility {
                    source: mire,
                    ability_index: 0,
                }
        })
        .unwrap_or_else(|| panic!("the search is not on offer at all: {options:?}"));
    assert!(
        !options[search].mana,
        "the fetch was read as a mana ability"
    );
    assert!(
        !options[search].tap_only,
        "a sacrifice and a life are not paid out of the card"
    );

    // And the click path reaches it. The Lantern gives the land a second,
    // granted ability, so this is now honestly a menu of two — which is the
    // right answer and was never the reported one: before this, the click
    // silently tapped for mana.
    activate_card(&mut duel, mire);
    assert_eq!(
        duel.ability_menu,
        Some(mire),
        "two abilities on one land are a menu"
    );
    for _ in 0..search {
        assert!(ability_menu_keys(
            Fired::of_actions(&[Action::CursorDown]),
            &mut duel
        ));
    }
    assert!(ability_menu_keys(
        Fired::of_actions(&[Action::Confirm]),
        &mut duel
    ));
    // Armed, not sent: sacrificing a land and paying a life is irreversible.
    assert!(duel.outbox().is_empty(), "picking sacrificed the land");
    assert!(armed_keys(Fired::of_actions(&[Action::Confirm]), &mut duel));
    assert_eq!(
        duel.outbox(),
        [PlayerAction::ActivateAbility {
            source: mire,
            ability_index: 0,
        }],
        "the fetchland tapped for mana instead of searching"
    );
}

/// The owner's fourth point, end to end: **the card taps only once the mana
/// has been chosen.**
///
/// Reported as *"Bei Karten die reines Mana generieren (aber halt Wahl des
/// Spielers). Da sollte lieber so ein kleines Pergament-Stil Dialog direkt
/// unter der Karte aufploppen wo der Spieler dann das Mana-Symbol wählen
/// kann"*, and then, in the same breath: *"die Karte wird erst dann getappt,
/// wenn das Mana ausgewählt wurde"*.
///
/// What it replaces is the one-option short-circuit in `activate_card`: a
/// Tundra offered exactly one thing, so the click fired it, the land tapped,
/// and the colour was asked afterwards in the prompt bar at the bottom of the
/// screen. The assertion that carries the whole report is the one in the
/// middle — the land is **untapped** while the bubble stands open.
#[test]
fn a_land_that_asks_which_colour_is_not_tapped_until_the_answer() {
    use baylee_client::input::{activate_card, sheet_digit};
    use baylee_client::{Duel, advance_mana_run};
    use baylee_client_core::interaction::Interaction;

    let mut table = Table::open_with(&bubble_preset());
    table.walk_to_main();

    let tundra = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Tundra")
        .expect("the land starts on the table")
        .id;

    let mut duel = Duel::default();
    let refresh = |duel: &mut Duel, table: &Table| {
        duel.view = Some(table.view().clone());
        duel.interaction = Some(Interaction::new(
            table.pending.clone().expect("priority"),
            PlayerId::new(0),
        ));
        baylee_client::rebuild_board(duel);
    };
    let tapped = |table: &Table| {
        table
            .view()
            .battlefield
            .iter()
            .find(|o| o.id == tundra)
            .expect("still on the table")
            .status
            .contains(baylee_view::ObjectStatus::TAPPED)
    };
    refresh(&mut duel, &table);
    assert!(!tapped(&table), "it starts untapped");

    // The click opens the bubble and sends nothing. Two pips, because a
    // Tundra makes two colours.
    activate_card(&mut duel, tundra);
    assert_eq!(duel.ability_menu, Some(tundra), "the bubble is open");
    assert!(duel.outbox().is_empty(), "and nothing reached the wire");
    assert!(!tapped(&table), "**the land is still untapped**");

    let options = baylee_client::abilities::options(
        baylee_client_core::Lang::En,
        duel.view.as_ref().expect("a view"),
        duel.interaction.as_ref().expect("priority"),
        tundra,
    );
    assert!(
        baylee_client::abilities::pouring(&options),
        "every row is a pour, so the sheet draws pips: {options:?}"
    );
    assert_eq!(
        options
            .iter()
            .filter_map(|o| o.pour.map(|p| p.color))
            .collect::<Vec<_>>(),
        vec![
            baylee_core::mana::ManaColor::White,
            baylee_core::mana::ManaColor::Blue
        ],
        "white then blue, in `ManaColor` order"
    );
    // The counter-test to Round eight's prefix: a Tundra pours one mana per
    // press, so there is no multiplier to draw and drawing one would be a
    // claim about the card.
    assert_eq!(
        baylee_client::abilities::bubble_prefix(
            duel.view.as_ref().expect("a view"),
            tundra,
            &options
        ),
        None,
        "a land's pip is one mana and says so by saying nothing"
    );

    // The second pip: blue. One press, and the activation goes out with the
    // colour already decided.
    assert!(sheet_digit(&mut duel, '2'), "the digit names a pip");
    assert!(duel.mana_run.is_some(), "which starts a one-step run");
    assert_eq!(duel.ability_menu, None, "and puts the bubble away");

    for _ in 0..8 {
        for action in duel.take_outbox() {
            table.submit(action);
        }
        refresh(&mut duel, &table);
        if duel.mana_run.is_none() {
            break;
        }
        advance_mana_run(&mut duel);
    }

    assert_eq!(duel.last_error, None, "the run finished without aborting");
    assert!(duel.mana_run.is_none(), "and finished");
    assert!(tapped(&table), "*now* the land is tapped");
    let pool = table
        .view()
        .seat(PlayerId::new(0))
        .expect("own seat")
        .mana_pool;
    assert_eq!(
        (pool.blue, pool.white),
        (1, 0),
        "one blue, which is the pip that was pressed: {pool:?}"
    );
    assert!(
        table.view().stack.is_empty(),
        "a mana ability never uses the stack (CR 605.3b)"
    );
}
