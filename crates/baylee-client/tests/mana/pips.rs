//! The pips: mana drawn as a row of coloured dots rather than a sentence.
//!
//! A permanent whose ability is just mana is drawn as its pips; one that
//! also does something else gets them as a header; one whose second tap the
//! pips cannot stand for keeps its printed sentence. And the keyboard walks
//! them.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// The same bubble on a creature, and on the ability no planner can read.
///
/// Harabaz Druid's `Add X mana of any one color` has an `Amount::CountOf`,
/// so `mana_shape` refuses it and `manasources::sources` has never seen it —
/// it was the empty `↺` row on the sheet. `mana_offer` reads the colour
/// question alone, which is all a bubble asks, and the owner's addendum said
/// the dialog is for *"Artefakte und Kreaturen, die nur Mana Ability haben"*.
#[test]
fn a_creature_whose_only_ability_is_mana_gets_the_same_five_pips() {
    use baylee_client::input::activate_card;
    use baylee_client::{Duel, abilities, manasources};
    use baylee_client_core::interaction::Interaction;
    use baylee_core::mana::ManaColor;

    let mut table = Table::open_with(&bubble_preset());
    table.walk_to_main();

    let druid = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Harabaz Druid")
        .expect("the creature starts on the table")
        .id;

    // The premise, and the reason this needed a fourth reading at all: the
    // planner cannot count what this makes, so it is in no `Source` list.
    assert!(
        !manasources::sources(table.view(), table.legal())
            .iter()
            .any(|s| s.id == druid),
        "no planner can read an amount that is a count of the battlefield"
    );

    let mut duel = Duel::default();
    duel.view = Some(table.view().clone());
    duel.interaction = Some(Interaction::new(
        table.pending.clone().expect("priority"),
        PlayerId::new(0),
    ));
    baylee_client::rebuild_board(&mut duel);

    let options = abilities::options(
        baylee_client_core::Lang::En,
        duel.view.as_ref().expect("a view"),
        duel.interaction.as_ref().expect("priority"),
        druid,
    );
    assert!(abilities::pouring(&options), "a bubble: {options:?}");
    assert_eq!(
        options
            .iter()
            .filter_map(|o| o.pour.map(|p| p.color))
            .collect::<Vec<_>>(),
        vec![
            ManaColor::White,
            ManaColor::Blue,
            ManaColor::Black,
            ManaColor::Red,
            ManaColor::Green
        ],
        "any one colour, in WUBRG order"
    );
    // Round eight's third point reaches here too, and by the same rule rather
    // than by a second one: five pips, all of a tap whose pour is not a
    // number, so the bubble says how *many* as well as which.
    assert_eq!(
        abilities::bubble_prefix(duel.view.as_ref().expect("a view"), druid, &options).as_deref(),
        Some("X×"),
        "the Druid alone is the same question as the Druid under a Guide"
    );

    activate_card(&mut duel, druid);
    assert_eq!(duel.ability_menu, Some(druid), "the click opens the bubble");
    assert!(duel.outbox().is_empty(), "and taps nothing");
}

/// The owner's sixth point: a permanent that makes mana **and** does
/// something else keeps its sheet, with the colours as a header on it.
///
/// Reported as *"Wenn es sowas wie Harabaz Druid in der Kombi ist, oder wie
/// beim Gates of Istfell: Dann kommt der normale Abilities Dialog ABER er hat
/// eine Besonderheit. Die erste Zeile ist quasi sowas wie der Mana-Dialog,
/// dort stehen zentriert die Farben, die es produzieren kann (oder auch
/// Farblos wie beim Artefakt) und via Klick ist es dann das. Nur dass man eben
/// auch die anderen Abilities noch zur Auswahl hat."*
///
/// Two halves, and the second is the one that could quietly go wrong: the pips
/// carry **no digit**, so `1` is the written row and not the `{C}`. A list that
/// numbered the pips would put a `2` on the only keycap this sheet draws.
#[test]
fn a_permanent_that_also_makes_mana_gets_the_pips_as_a_header() {
    use baylee_client::input::{activate_card, sheet_digit};
    use baylee_client::{Duel, abilities};
    use baylee_client_core::interaction::Interaction;

    let mut preset = bubble_preset();
    preset.seats[0].starting_battlefield = vec![entry(HOMEWARD)];
    let mut table = Table::open_with(&preset);
    table.walk_to_main();

    let path = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Homeward Path")
        .expect("the land starts on the table")
        .id;

    let mut duel = Duel::default();
    duel.view = Some(table.view().clone());
    duel.interaction = Some(Interaction::new(
        table.pending.clone().expect("priority"),
        PlayerId::new(0),
    ));
    baylee_client::rebuild_board(&mut duel);

    let options = abilities::options(
        baylee_client_core::Lang::En,
        duel.view.as_ref().expect("a view"),
        duel.interaction.as_ref().expect("priority"),
        path,
    );
    assert!(
        !abilities::pouring(&options),
        "not a bubble — it does something besides make mana: {options:?}"
    );
    let split = abilities::Split::of(&options);
    assert_eq!(
        (split.pips, split.rows),
        (1, 1),
        "one pip for the {{C}}, one written row for the rest: {options:?}"
    );
    assert_eq!(
        options[0].pour.map(|pour| pour.color),
        Some(baylee_core::mana::ManaColor::Colorless),
        "and a colourless pip is a pip — `of_mana` answers what `of_color` \
         cannot (CR 106.1b)"
    );

    activate_card(&mut duel, path);
    assert_eq!(duel.ability_menu, Some(path), "the sheet opens");
    assert!(duel.outbox().is_empty(), "and sends nothing");

    // The one keycap on this sheet says `1`, and it is on the sentence. The
    // pip above it carries no digit at all.
    assert!(
        sheet_digit(&mut duel, '1'),
        "the digit names the written row"
    );
    assert!(duel.mana_run.is_none(), "which is not a pour");
    assert_eq!(
        duel.outbox(),
        [PlayerAction::ActivateAbility {
            source: path,
            ability_index: 1,
        }],
        "`1` sent the sentence, not the {{C}}"
    );
}

/// Round seven's first point: Harabaz Druid **under a Great Divide Guide**,
/// where it has two mana abilities and one of them had gone missing.
///
/// Reported as *"er hat in der Konstilazion zwei Mana Abilities. 1x die
/// geerbte vom Great Divide Guide und dann die eigene, mit X = count(allies),
/// diese ist gerade 'verloren gegangen'"*. Both taps make all five colours, so
/// the colour-wise union has one pip per colour to share between them and the
/// loser used to be dropped from the list outright.
///
/// Two claims, and they are the two halves of the fix. The pips stand for the
/// **grant**, because a pip promises one mana of the colour pressed and the
/// Druid's own X is a count of the battlefield nothing this side of the engine
/// reads. And the Druid's own ability is a written row saying what it makes,
/// because a tap the header does not stand for keeps its sentence.
#[test]
fn a_second_mana_tap_the_pips_cannot_stand_for_keeps_its_sentence() {
    use baylee_client::input::{activate_card, sheet_digit};
    use baylee_client::{Duel, abilities};
    use baylee_client_core::interaction::Interaction;
    use baylee_engine::choice::GRANTED_ABILITY;

    let mut preset = bubble_preset();
    preset.seats[0].starting_battlefield = vec![entry(SQUAD), entry(HARABAZ)];
    let mut table = Table::open_with(&preset);
    table.walk_to_main();

    let druid = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Harabaz Druid")
        .expect("the creature starts on the table")
        .id;

    let mut duel = Duel::default();
    duel.view = Some(table.view().clone());
    duel.interaction = Some(Interaction::new(
        table.pending.clone().expect("priority"),
        PlayerId::new(0),
    ));
    baylee_client::rebuild_board(&mut duel);

    // The premise: the engine is offering both, the Druid's own ability at
    // index 0 and the Guide's grant under the synthetic one.
    let legal = table.legal();
    assert!(legal.abilities.contains(&(druid, 0)), "its own {legal:?}");
    assert!(
        legal.abilities.contains(&(druid, GRANTED_ABILITY)),
        "and the one the Guide hands every Ally: {legal:?}"
    );

    let options = abilities::options(
        baylee_client_core::Lang::En,
        duel.view.as_ref().expect("a view"),
        duel.interaction.as_ref().expect("priority"),
        druid,
    );
    let split = abilities::Split::of(&options);
    assert_eq!(
        (split.pips, split.rows),
        (5, 1),
        "five colours as a header, and the ability they are not: {options:?}"
    );
    assert!(
        options[..split.pips].iter().all(|option| option
            .pour
            .as_ref()
            .is_some_and(|pour| pour.step.tap == manaplan::Tap::Ability(GRANTED_ABILITY))),
        "every pip is the grant, whose pour is one mana: {options:?}"
    );
    assert_eq!(
        options[0].action,
        PlayerAction::ActivateAbility {
            source: druid,
            ability_index: GRANTED_ABILITY,
        },
        "and presses the activation the engine offered it under"
    );

    let own = &options[split.option(0)];
    assert!(own.pour.is_none(), "the written row is no pip: {own:?}");
    assert_eq!(
        own.action,
        PlayerAction::ActivateAbility {
            source: druid,
            ability_index: 0,
        },
        "and it is the Druid's own ability"
    );
    assert_eq!(
        own.label, "Tap for X mana (any color)",
        "which says what it makes rather than repeating its own cost"
    );
    assert_eq!(own.cost.as_deref(), Some("{T}"), "the cost column: {own:?}");
    // Round eight's second point: *"ich fände es schöner, wenn hier der
    // Original Text angezeigt wird, statt einem customizierten"*. The label
    // above is a fallback and is what this row drew until the line table
    // learned to place a mana ability at all; the Druid prints one sentence
    // and this is it.
    assert_eq!(
        own.printed,
        Some(baylee_view::StackText {
            face: 0,
            line: 0,
            of: 1,
        }),
        "the card's own sentence, not the label: {own:?}"
    );

    // And the one keycap this sheet draws is on that row, the pips carrying
    // no digit.
    activate_card(&mut duel, druid);
    assert_eq!(duel.ability_menu, Some(druid), "the click opens the sheet");
    assert!(duel.outbox().is_empty(), "and taps nothing");
    assert!(
        sheet_digit(&mut duel, '1'),
        "the digit names the written row"
    );
    // Round eight's third point: *"wenn man den Effekt auswählt, dann
    // verschwindet der Abilities Dialog und es wird wieder der Mana Dialog
    // angezeigt"*. The press does not send — this tap makes X mana of **one**
    // colour, and which colour is the only question it has left.
    assert!(duel.mana_run.is_none(), "which is not a pour");
    assert!(duel.outbox().is_empty(), "and still taps nothing");
    assert_eq!(duel.ability_menu, Some(druid), "the sheet stands");
    assert_eq!(duel.asking_tap(), Some(0), "as that tap's own bubble");
}

/// Round eight's third point, end to end: the written row opens a bubble of
/// its own, and one pip of it pours **two green**.
///
/// Reported as *"wenn man den Effekt auswählt, dann verschwindet der Abilities
/// Dialog und es wird wieder der Mana Dialog angezeigt, allerdings mit einem
/// Präfix … Entweder Xx(Symbol Auswahl), oder für X direkt ausgerechnet die
/// Anzahl an Allys die ich kontrolliere."*
///
/// The last assertion is also the engine half's proof. Harabaz Druid prints
/// "Add X mana of any **one** color" and was written with
/// `Effect::mana_combination`, which the engine reads as a colour pick per
/// mana — so two Allies used to be two questions and could come out `{W}{U}`.
/// Two green out of one press is both halves at once.
#[test]
fn the_x_row_opens_a_bubble_whose_pip_pours_all_of_x_in_one_colour() {
    use baylee_client::input::{activate_card, sheet_digit};
    use baylee_client::{Duel, abilities, advance_mana_run};
    use baylee_client_core::interaction::Interaction;

    let mut preset = bubble_preset();
    preset.seats[0].starting_battlefield = vec![entry(SQUAD), entry(HARABAZ)];
    let mut table = Table::open_with(&preset);
    table.walk_to_main();

    let druid = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Harabaz Druid")
        .expect("the creature starts on the table")
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
    refresh(&mut duel, &table);

    activate_card(&mut duel, druid);
    assert!(sheet_digit(&mut duel, '1'), "the written row");
    assert_eq!(duel.asking_tap(), Some(0), "steps into its own tap");

    // What the sheet is now: five pips, no written rows, every one of them
    // the Druid's own ability — and a prefix, because one press of it pours a
    // number this side of the wire cannot count.
    let options = abilities::options_for(
        baylee_client_core::Lang::En,
        duel.view.as_ref().expect("a view"),
        duel.interaction.as_ref().expect("priority"),
        druid,
        duel.asking_tap(),
    );
    let split = abilities::Split::of(&options);
    assert_eq!((split.pips, split.rows), (5, 0), "a bubble: {options:?}");
    assert!(
        abilities::pouring(&options),
        "which is what the sheet draws as pips alone"
    );
    assert!(
        options.iter().all(|option| option
            .pour
            .as_ref()
            .is_some_and(|pour| pour.step.tap == manaplan::Tap::Ability(0))),
        "all of the Druid's own tap, not the Guide's grant: {options:?}"
    );
    assert_eq!(
        abilities::bubble_prefix(duel.view.as_ref().expect("a view"), druid, &options).as_deref(),
        Some("X×"),
        "and says so before them"
    );

    // Green is the fifth pip in `ManaColor` order, and `5` is its digit: a
    // bubble has no written rows, so the digits count the pips.
    assert!(sheet_digit(&mut duel, '5'), "the digit names the green pip");
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

    let pool = table
        .view()
        .seat(PlayerId::new(0))
        .expect("own seat")
        .mana_pool;
    assert_eq!(
        (pool.green, pool.white, pool.blue, pool.black, pool.red),
        (2, 0, 0, 0, 0),
        "two Allies, two mana, both of the one colour pressed: {pool:?}"
    );
}

/// Round seven's third point: the mana pips are walked by the two horizontal
/// keys, and either of them reaches the strip from a written row at once.
///
/// Reported as *"bei den Mana Symbolen, die kann man mit A und D navigieren.
/// Wenn man A oder D navigiert, switcht es sofort zur Mana Zeile, mit W und S
/// wie gehabt hoch und runter."* The sheet is the only one in the client with
/// two directions on it — a centred row of pips above a column of sentences —
/// and all four keys used to step the flat list by one, so `D` on a pip moved
/// sideways and `D` on a sentence moved down.
#[test]
fn the_horizontal_keys_walk_the_pips_and_jump_to_them() {
    use baylee_client::input::{ability_menu_keys, activate_card};
    use baylee_client::keys::Fired;
    use baylee_client::{Duel, abilities};
    use baylee_client_core::interaction::Interaction;
    use baylee_client_core::prefs::Action;

    let mut preset = bubble_preset();
    preset.seats[0].starting_battlefield = vec![entry(SQUAD), entry(HARABAZ)];
    let mut table = Table::open_with(&preset);
    table.walk_to_main();

    let druid = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Harabaz Druid")
        .expect("the creature starts on the table")
        .id;

    let mut duel = Duel::default();
    duel.view = Some(table.view().clone());
    duel.interaction = Some(Interaction::new(
        table.pending.clone().expect("priority"),
        PlayerId::new(0),
    ));
    baylee_client::rebuild_board(&mut duel);

    activate_card(&mut duel, druid);
    assert_eq!(duel.ability_menu, Some(druid), "the sheet opens");
    let options = abilities::options(
        baylee_client_core::Lang::En,
        duel.view.as_ref().expect("a view"),
        duel.interaction.as_ref().expect("priority"),
        druid,
    );
    let split = abilities::Split::of(&options);
    assert_eq!((split.pips, split.rows), (5, 1), "five pips and a sentence");

    let key = |duel: &mut Duel, action| ability_menu_keys(Fired::of_actions(&[action]), duel);

    // Down walks everything there is, header included, exactly as before.
    assert!(key(&mut duel, Action::CursorDown));
    assert_eq!(duel.ability_pick, 1, "the second pip");

    // Right walks the header and wraps inside it, without ever reaching the
    // sentence: five pips, not six options.
    for expected in [2, 3, 4, 0] {
        assert!(key(&mut duel, Action::CursorRight));
        assert_eq!(duel.ability_pick, expected, "the header is its own ring");
    }
    assert!(key(&mut duel, Action::CursorLeft));
    assert_eq!(duel.ability_pick, 4, "and the other way round it");

    // Onto the written row, by the vertical key that owns the column.
    assert!(key(&mut duel, Action::CursorDown));
    assert_eq!(duel.ability_pick, split.option(0), "the sentence");

    // And one horizontal press arrives on the strip, at the end it was
    // heading for — this is the half the owner asked for by name.
    assert!(key(&mut duel, Action::CursorRight));
    assert_eq!(duel.ability_pick, 0, "rightwards enters at the first pip");
    assert!(
        key(&mut duel, Action::CursorUp),
        "up off the first pip is the top of the whole column"
    );
    assert_eq!(
        duel.ability_pick,
        split.option(0),
        "which wraps round to the sentence"
    );
    assert!(key(&mut duel, Action::CursorLeft));
    assert_eq!(duel.ability_pick, 4, "leftwards enters at the last");
}
