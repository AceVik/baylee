//! A cast paid by a land that hurts: a painland's coloured tap deals its
//! controller 1 damage beside the mana, City of Brass deals 1 whenever it
//! becomes tapped. The owner's report (08.10.2026): "whenever the player
//! casts something and this land gets tapped, the cast is interrupted and the
//! mana is auto-tapped".
//!
//! Every test clicks the card the way a player does — arm, confirm — and lets
//! the run spend its plan against a real `LocalHost`.

use super::*;

/// Adarkar Wastes: `{T}: Add {C}` and `{T}: Add {W} or {U}`, 1 damage to you.
const ADARKAR: &str = "d5ad26cc-2bdb-46b7-b8bf-dd099d5fa09b";
/// City of Brass: any colour, and 1 damage whenever it becomes tapped.
const BRASS: &str = "f25351e3-539b-4bbc-b92d-6480acf4d722";
/// Serra Zealot, `{W}`.
const ZEALOT: &str = "989a7353-8d3d-4ea2-ab5e-8535d95dddae";
/// Crystalline Sliver, `{W}{U}`: a creature, so cast only with an empty stack.
const SLIVER: &str = "ba3aa1eb-722a-47d3-83be-96daddb50265";
/// Living Artifact: "Whenever you're dealt damage, put that many vitality
/// counters on this Aura."
const LIVING_ARTIFACT: &str = "4ff9af56-ac18-4966-9e48-183e1ca1c2d0";
/// Sol Ring, the artifact it enchants.
const SOL_RING: &str = "6ad8011d-3471-4369-9d68-b264cc027487";

/// Seat 0 with `hand` and these lands, in its first main phase.
fn table_with(hand: &[&str], lands: &[&str]) -> Table {
    let mut preset = white_preset(hand, 0);
    preset.seats[0].starting_battlefield = lands.iter().map(|l| entry(l)).collect();
    let mut table = Table::open_with(&preset);
    table.walk_to_main();
    table
}

fn life(table: &Table) -> i32 {
    table.view().seat(PlayerId::new(0)).expect("own seat").life
}

fn on_stack_or_battlefield(table: &Table, name: &str) -> bool {
    table
        .view()
        .battlefield
        .iter()
        .chain(table.view().stack.iter())
        .any(|o| o.name == name)
}

/// Clicks `name` in hand twice (arm, confirm) and plays the run out, one
/// engine round trip per step. Returns the run's refusal, if it had one.
fn click_to_cast(table: &mut Table, name: &str) -> Option<String> {
    use baylee_client::Duel;
    use baylee_client::input::activate_card;
    let card = id_in_hand(table, name);
    let mut duel = Duel::default();
    refresh(&mut duel, table);
    activate_card(&mut duel, card);
    activate_card(&mut duel, card);
    for _ in 0..16 {
        for action in duel.take_outbox() {
            table.submit(action);
        }
        refresh(&mut duel, table);
        if duel.mana_run.is_none() {
            break;
        }
        baylee_client::advance_mana_run(&mut duel);
    }
    for action in duel.take_outbox() {
        table.submit(action);
    }
    duel.last_error.map(|e| format!("{e:?}"))
}

/// One white pip, a Plains and a painland: the Plains pays, nobody is hurt.
#[test]
fn a_white_pip_is_paid_by_the_plains_and_not_the_painland() {
    let mut table = table_with(&[ZEALOT], &[PLAINS, ADARKAR]);
    let before = life(&table);
    assert_eq!(click_to_cast(&mut table, "Serra Zealot"), None);
    assert!(on_stack_or_battlefield(&table, "Serra Zealot"));
    assert_eq!(life(&table), before, "the Plains paid");
}

/// `{W}{U}` from a Plains and Adarkar Wastes: the Wastes taps for blue, deals
/// its damage, and the spell is cast.
#[test]
fn white_and_blue_from_a_plains_and_a_painland_casts_the_spell() {
    let mut table = table_with(&[SLIVER], &[PLAINS, ADARKAR]);
    let before = life(&table);
    let refusal = click_to_cast(&mut table, "Crystalline Sliver");
    assert_eq!(refusal, None);
    assert!(on_stack_or_battlefield(&table, "Crystalline Sliver"));
    assert_eq!(life(&table), before - 1, "the Wastes' blue cost a life");
}

/// The colour tapped by hand first, then the card clicked: the run spends
/// the blue already floating and taps the Plains, and the cast goes through.
#[test]
fn a_painland_tapped_by_hand_for_its_colour_does_not_stop_the_cast() {
    let mut table = table_with(&[SLIVER], &[PLAINS, ADARKAR]);
    let wastes = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Adarkar Wastes")
        .expect("the Wastes")
        .id;
    let before = life(&table);
    table.submit(PlayerAction::ActivateAbility {
        source: wastes,
        ability_index: 1,
    });
    table.submit(PlayerAction::ChooseColor(
        baylee_core::mana::ManaColor::Blue,
    ));
    assert_eq!(life(&table), before - 1);
    assert_eq!(click_to_cast(&mut table, "Crystalline Sliver"), None);
    assert!(on_stack_or_battlefield(&table, "Crystalline Sliver"));
}

/// With a clean land for the colour, City of Brass is left untapped. A guard,
/// not the proof of its price: the planner's breadth order already passes
/// over a five-colour land here, and `city_of_brass_is_priced_by_its_own_tap_trigger`
/// is the test that reads the price.
#[test]
fn city_of_brass_is_passed_over_when_a_clean_land_makes_the_colour() {
    let mut table = table_with(&[SLIVER], &[ISLAND, PLAINS, BRASS]);
    let before = life(&table);
    assert_eq!(click_to_cast(&mut table, "Crystalline Sliver"), None);
    assert!(on_stack_or_battlefield(&table, "Crystalline Sliver"));
    assert_eq!(life(&table), before, "the City was never tapped");
}

/// The owner's report (08.10.2026), and the half a planner could not reach:
/// City of Brass is the only white here, and its damage is a triggered
/// ability. Tapped before the cast, as this client used to pay, its trigger
/// went on the stack first and a creature could no longer be cast
/// (`PlanSpellRefused`, the lands tapped and the mana floating). The card
/// is now cast first and paid for in the window its cast opens (CR 601.2g),
/// so the trigger waits until the creature is cast (CR 601.2i) and goes on
/// the stack above it.
#[test]
fn city_of_brass_as_the_only_colour_pays_for_a_creature_cast_first() {
    let mut table = table_with(&[SLIVER], &[ISLAND, BRASS]);
    let before = life(&table);
    assert_eq!(click_to_cast(&mut table, "Crystalline Sliver"), None);
    let stack: Vec<&str> = table.view().stack.iter().map(|o| o.name.as_str()).collect();
    assert_eq!(
        stack.len(),
        2,
        "the creature and the City's trigger: {stack:?}"
    );
    assert_eq!(stack[0], "Crystalline Sliver", "the creature underneath");
    assert_eq!(life(&table), before, "the trigger has not resolved yet");
}

/// Living Artifact ("Whenever you're dealt damage") on a Sol Ring, and
/// Adarkar Wastes paying the blue: the painland's damage sets off the Aura.
/// Paid before the cast, the Aura's trigger stopped it; paid inside it, the
/// trigger waits above the creature.
#[test]
fn a_painland_beside_a_damage_trigger_pays_for_a_creature_cast_first() {
    let mut preset = white_preset(&[LIVING_ARTIFACT, SLIVER], 0);
    preset.seats[0].starting_battlefield = [FOREST, SOL_RING, PLAINS, ADARKAR]
        .iter()
        .map(|l| entry(l))
        .collect();
    let mut table = Table::open_with(&preset);
    table.walk_to_main();
    let named = |table: &Table, name: &str| {
        table
            .view()
            .battlefield
            .iter()
            .find(|o| o.name == name)
            .unwrap_or_else(|| panic!("{name} on the table"))
            .id
    };
    let (forest, ring) = (named(&table, "Forest"), named(&table, "Sol Ring"));
    table.submit(PlayerAction::ActivateManaAbility { source: forest });
    table.submit(PlayerAction::CastSpell {
        card: id_in_hand(&table, "Living Artifact"),
    });
    table.submit(PlayerAction::ChooseTargets {
        objects: vec![ring],
        players: vec![],
    });
    while !table.view().stack.is_empty() {
        table.submit(PlayerAction::PassPriority);
    }
    assert!(
        table
            .view()
            .battlefield
            .iter()
            .any(|o| o.name == "Living Artifact")
    );

    let before = life(&table);
    assert_eq!(click_to_cast(&mut table, "Crystalline Sliver"), None);
    let stack: Vec<&str> = table.view().stack.iter().map(|o| o.name.as_str()).collect();
    assert_eq!(
        stack.len(),
        2,
        "the creature and the Aura's trigger: {stack:?}"
    );
    assert_eq!(stack[0], "Crystalline Sliver");
    assert_eq!(life(&table), before - 1, "the Wastes' blue cost a life");
}

/// [`click_to_cast`], and every action it submitted, in order.
fn click_to_cast_recorded(table: &mut Table, name: &str) -> (Option<String>, Vec<PlayerAction>) {
    use baylee_client::Duel;
    use baylee_client::input::activate_card;
    let card = id_in_hand(table, name);
    let mut duel = Duel::default();
    let mut sent = Vec::new();
    refresh(&mut duel, table);
    activate_card(&mut duel, card);
    activate_card(&mut duel, card);
    for _ in 0..16 {
        for action in duel.take_outbox() {
            sent.push(action.clone());
            table.submit(action);
        }
        refresh(&mut duel, table);
        if duel.mana_run.is_none() {
            break;
        }
        baylee_client::advance_mana_run(&mut duel);
    }
    for action in duel.take_outbox() {
        sent.push(action.clone());
        table.submit(action);
    }
    (duel.last_error.map(|e| format!("{e:?}")), sent)
}

/// A spell paid in its own window goes out as ONE `PayWindow`: with only a
/// painland untapped, Serra Zealot's `{W}` is paid by the Wastes' coloured
/// tap, and that tap (with the colour it asks) travels inside the window's
/// single action, never as a separate top-level activation before it. The
/// damage is dealt once, when the whole payment is applied.
#[test]
fn a_painland_pays_a_spell_inside_one_pay_window() {
    let mut table = table_with(&[ZEALOT], &[ADARKAR]);
    let wastes = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Adarkar Wastes")
        .expect("the Wastes")
        .id;
    let before = life(&table);

    let (refusal, sent) = click_to_cast_recorded(&mut table, "Serra Zealot");
    assert_eq!(refusal, None, "sent: {sent:?}");
    assert!(on_stack_or_battlefield(&table, "Serra Zealot"));
    assert_eq!(life(&table), before - 1, "the Wastes' white cost a life");

    let windows: Vec<&Vec<PlayerAction>> = sent
        .iter()
        .filter_map(|action| match action {
            PlayerAction::PayWindow { steps } => Some(steps),
            _ => None,
        })
        .collect();
    assert_eq!(windows.len(), 1, "exactly one PayWindow in {sent:?}");
    let steps = windows[0];
    assert!(
        matches!(
            steps.first(),
            Some(PlayerAction::ActivateAbility { source, .. }) if *source == wastes
        ),
        "the Wastes' activation leads the window: {steps:?}"
    );
    if let Some(PlayerAction::ActivateAbility { ability_index, .. }) = steps.first() {
        assert_eq!(*ability_index, 1, "the coloured mode, not the free {{C}}");
    }
    assert_eq!(
        steps.get(1),
        Some(&PlayerAction::ChooseColor(
            baylee_core::mana::ManaColor::White
        )),
        "the colour answer follows its tap inside the same action: {steps:?}"
    );
    assert_eq!(steps.len(), 2, "and nothing else is in the window");

    let loose: Vec<&PlayerAction> = sent
        .iter()
        .filter(|action| {
            matches!(
                action,
                PlayerAction::ActivateAbility { .. }
                    | PlayerAction::ActivateManaAbility { .. }
                    | PlayerAction::ChooseColor(_)
            )
        })
        .collect();
    assert!(loose.is_empty(), "taps sent outside the window: {loose:?}");
}

/// The written row of a painland's coloured tap, pressed inside a payment
/// window, stages nothing and sends nothing: it steps into the tap's colour
/// bubble, and the bubble's pip stages the tap. Only Go sends it, and only
/// then does the land tap and deal its damage.
///
/// Regression for a live run (13.10.2026): with a fixed-cost window open
/// (CR 605.3a, the window a cast opens, paid by staging taps locally and
/// sending one `PayWindow` on Go), pressing Adarkar Wastes' written row
/// "{T}: Add {W} or {U}. This land deals 1 damage to you." sent
/// `ActivateAbility { ability_index: 1 }` at once: the land tapped, the
/// damage was dealt and the engine asked `ChooseColor`, all before the
/// player had said Go. On the old code the "sent nothing" assertion after the
/// row press fails (the outbox holds that activation); with only the
/// `stage_in_window` half of the repair, the pips of the bubble are missing
/// and the Blue pip lookup fails instead.
#[test]
fn a_painlands_written_row_in_a_payment_window_waits_for_go() {
    use baylee_client::Duel;
    use baylee_client::input::activate_card;
    use baylee_client_core::manaplan::{Step, Tap};
    use baylee_core::mana::ManaColor;

    let mut table = table_with(&[ZEALOT], &[ADARKAR]);
    let wastes = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Adarkar Wastes")
        .expect("the Wastes")
        .id;
    let before = life(&table);

    // The window, opened by the cast alone: the client's own run does not pay.
    let zealot = id_in_hand(&table, "Serra Zealot");
    assert!(
        table.legal().payable.contains(&zealot),
        "the Zealot is payable off the Wastes"
    );
    table.submit(PlayerAction::CastSpell { card: zealot });
    let mut duel = Duel::default();
    refresh(&mut duel, &table);
    assert!(duel.paying(), "the cast opened a fixed-cost payment window");

    // The sheet of the land, and nothing yet on the wire.
    activate_card(&mut duel, wastes);
    assert_eq!(duel.ability_menu, Some(wastes), "the land opens its sheet");
    assert!(
        duel.take_outbox().is_empty(),
        "opening a sheet sends nothing"
    );

    // The written row of ability 1, pressed by its digit.
    press_the_written_row(&mut duel, wastes, 1);

    assert_eq!(
        duel.take_outbox(),
        Vec::<PlayerAction>::new(),
        "the row asks its colour on a bubble; it sends nothing before Go"
    );
    assert_eq!(duel.ability_menu, Some(wastes), "the sheet is still up");
    assert_eq!(duel.ability_tap, Some(1), "as the bubble of ability 1");
    table.drain();
    assert!(
        !table
            .view()
            .battlefield
            .iter()
            .find(|o| o.id == wastes)
            .expect("the Wastes")
            .status
            .contains(baylee_view::ObjectStatus::TAPPED),
        "the Wastes is untapped"
    );
    assert_eq!(life(&table), before, "and nothing hurt");

    // The Blue pip of that bubble stages the tap with its colour.
    let blue = pip_row(&duel, wastes, baylee_core::mana::ManaColor::Blue);
    press_the_cursor_row(&mut duel, blue);
    assert_eq!(
        duel.take_outbox(),
        Vec::<PlayerAction>::new(),
        "a pip in a payment window stages; it sends nothing"
    );
    assert_eq!(
        duel.pay_staged.steps(),
        [Step {
            source: wastes,
            tap: Tap::Ability(1),
            color: Some(ManaColor::Blue),
        }],
        "exactly the Blue tap of ability 1 is staged"
    );
    assert_eq!(life(&table), before, "still nothing has happened");

    // Go: now, and only now, the window is paid and the damage is dealt.
    // Blue does not pay `{W}`, so stage White in its place first.
    duel.unstage(wastes);
    let white = pip_row(&duel, wastes, ManaColor::White);
    duel.ability_menu = Some(wastes);
    duel.ability_tap = Some(1);
    press_the_cursor_row(&mut duel, white);
    assert!(duel.staged_covers(), "White covers the Zealot's {{W}}");
    duel.confirm_payment();
    let sent = duel.take_outbox();
    assert!(
        matches!(sent.as_slice(), [PlayerAction::PayWindow { .. }]),
        "Go sends one PayWindow: {sent:?}"
    );
    for action in sent {
        table.submit(action);
    }
    assert_eq!(
        life(&table),
        before - 1,
        "the damage lands at Go, not before"
    );
    assert!(on_stack_or_battlefield(&table, "Serra Zealot"));
}

/// Moves the sheet's cursor down to `row` and presses confirm, as a player
/// does with the keys.
fn press_the_cursor_row(duel: &mut baylee_client::Duel, row: usize) {
    use baylee_client::input::ability_menu_keys;
    use baylee_client::keys::Fired;
    use baylee_client_core::prefs::Action;
    duel.ability_pick = 0;
    for _ in 0..32 {
        if duel.ability_pick == row {
            break;
        }
        ability_menu_keys(Fired::of_actions(&[Action::CursorDown]), duel);
    }
    assert_eq!(duel.ability_pick, row, "the cursor reached the row");
    ability_menu_keys(Fired::of_actions(&[Action::Confirm]), duel);
}

/// The row of the colour bubble of `object`'s ability 1 whose pip pours
/// `color`.
fn pip_row(
    duel: &baylee_client::Duel,
    object: baylee_core::ids::ObjectId,
    color: baylee_core::mana::ManaColor,
) -> usize {
    baylee_client::abilities::options_for(
        baylee_client_core::Lang::En,
        duel.view.as_ref().expect("view"),
        duel.interaction.as_ref().expect("interaction"),
        object,
        Some(1),
    )
    .iter()
    .position(|o| o.pour.is_some_and(|p| p.color == color))
    .unwrap_or_else(|| panic!("the bubble has a {color:?} pip"))
}

/// Presses the written row that activates `object`'s ability `index` by the
/// digit drawn on it.
fn press_the_written_row(
    duel: &mut baylee_client::Duel,
    object: baylee_core::ids::ObjectId,
    index: u32,
) {
    let options = baylee_client::abilities::options(
        baylee_client_core::Lang::En,
        duel.view.as_ref().expect("view"),
        duel.interaction.as_ref().expect("interaction"),
        object,
    );
    let (head, counted) = baylee_client::abilities::Split::of(&options).numbered();
    let row = options
        .iter()
        .position(|o| {
            o.action
                == PlayerAction::ActivateAbility {
                    source: object,
                    ability_index: index,
                }
        })
        .expect("the tap is a row of the sheet");
    assert!(row >= head, "it is a written row, not a pip");
    assert!(row - head < counted.min(9), "and on the first page");
    let digit = char::from_digit(u32::try_from(row - head + 1).expect("small"), 10).expect("digit");
    assert!(
        baylee_client::input::sheet_digit(duel, digit),
        "the digit names the row"
    );
}
