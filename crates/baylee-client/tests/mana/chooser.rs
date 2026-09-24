//! The ability chooser: what a permanent offers, and how that list is read.
//!
//! One permanent may be able to do several things, and the chooser is where
//! the client asks which. What is on the list, what each entry is called, and
//! what happens when it is answered, cancelled, or never opened because there
//! was only one entry.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// The manual half, which is the half that did not exist at all: a client
/// that cannot activate an ability cannot play a game, whatever it does with
/// mana automatically.
#[test]
fn a_permanent_offers_exactly_what_it_can_do_and_never_the_same_tap_twice() {
    use baylee_client::abilities;
    use baylee_client_core::interaction::Interaction;

    let mut table = Table::open();
    table.walk_to_main();

    let interaction = Interaction::new(table.pending.clone().expect("priority"), PlayerId::new(0));
    let forest = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Forest")
        .expect("a Forest on the table")
        .id;

    // One button, not two. A Forest is offered by the engine as the CR 305.6
    // shortcut *and* as the `{T}: Add {G}` printed on the card, and it can
    // still only be tapped once.
    let options = abilities::options(
        baylee_client_core::Lang::En,
        table.view(),
        &interaction,
        forest,
    );
    assert_eq!(options.len(), 1, "{options:?}");
    assert_eq!(options[0].label, "Tap for {G}");

    // And it is an action the engine takes.
    table.submit(options[0].action.clone());
    let pool = table
        .view()
        .seat(PlayerId::new(0))
        .expect("own seat")
        .mana_pool;
    assert_eq!(pool.green, 1, "the Forest made a green mana");

    // A card in hand is not a permanent and offers nothing to activate.
    let in_hand = table.view().hand.first().expect("a hand").id;
    assert!(
        abilities::options(
            baylee_client_core::Lang::En,
            table.view(),
            &interaction,
            in_hand
        )
        .is_empty()
    );
}

/// A permanent whose ability is not a mana ability at all.
///
/// Named by what it costs, which `abilities::options` once could not do: it
/// said "Ability 1", a label a player has to count out on the card. The cost
/// a player reads is the card's own printed head; the label keeps only the
/// symbols, for a row the card prints no sentence for.
#[test]
fn a_non_mana_ability_is_named_by_what_it_costs() {
    use baylee_client::abilities;
    use baylee_client_core::interaction::Interaction;

    let mut table = Table::open_with(&preset_with_a_fetchland());
    table.walk_to_main();

    let interaction = Interaction::new(table.pending.clone().expect("priority"), PlayerId::new(0));
    let mire = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Bloodstained Mire")
        .expect("the fetchland is on the table")
        .id;

    let options = abilities::options(
        baylee_client_core::Lang::En,
        table.view(),
        &interaction,
        mire,
    );
    assert_eq!(options.len(), 1, "{options:?}");
    assert_eq!(options[0].label, "{T}");
    let words = abilities::printed_words(None, table.view(), mire, &options[0])
        .expect("the fetchland prints this ability");
    assert_eq!(
        words.head.as_deref(),
        Some("{T}, Pay 1 life, Sacrifice this land"),
        "the card's own cost, in its own order and words"
    );
    assert_eq!(
        options[0].action,
        PlayerAction::ActivateAbility {
            source: mire,
            ability_index: 0,
        }
    );
}

/// …and the client's own click path activates it.
///
/// Through `activate_card`, which is where a pointer and the keyboard cursor
/// both end up: one option activates on the click that found it, so a player
/// never sees a menu of one. Before any of this the same click selected the
/// permanent for a choice that was not pending and did nothing at all.
#[test]
fn clicking_a_permanent_with_one_ability_activates_it() {
    use baylee_client::Duel;
    use baylee_client::input::activate_card;
    use baylee_client_core::interaction::Interaction;

    let mut table = Table::open_with(&preset_with_a_fetchland());
    table.walk_to_main();

    let mire = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Bloodstained Mire")
        .expect("the fetchland is on the table")
        .id;
    let life = table.view().seat(PlayerId::new(0)).expect("own seat").life;

    let mut duel = Duel::default();
    duel.view = Some(table.view().clone());
    duel.interaction = Some(Interaction::new(
        table.pending.clone().expect("priority"),
        PlayerId::new(0),
    ));

    activate_card(&mut duel, mire);
    assert!(
        duel.ability_menu.is_none(),
        "one option needs no chooser at all"
    );
    // Sacrificing a land and paying a life is irreversible, so the first
    // click arms and the second sends (`docs/design.md` §2.5). This is the
    // ability the design names as the reason the rule exists.
    assert!(
        duel.outbox().is_empty(),
        "the first click sacrificed the land"
    );
    activate_card(&mut duel, mire);
    let action = duel.outbox().first().cloned().expect("the click sent one");
    assert_eq!(
        action,
        PlayerAction::ActivateAbility {
            source: mire,
            ability_index: 0,
        }
    );

    // The engine agrees, which is the half a client cannot fake: the land
    // sacrifices itself and the life is paid.
    table.submit(action);
    assert!(
        !table
            .view()
            .battlefield
            .iter()
            .any(|o| o.name == "Bloodstained Mire"),
        "the fetchland sacrificed itself"
    );
    assert_eq!(
        table.view().seat(PlayerId::new(0)).expect("own seat").life,
        life - 1,
        "and the life was paid"
    );
}

/// The ability chooser can be answered without a pointer.
///
/// It could not, and that is the whole reason this test exists: opening the
/// menu on a permanent with two abilities put the keyboard in a room with one
/// door, `Esc`. Confirm did nothing, the cursor keys walked the table behind
/// the menu, and the only way to activate the second ability was the mouse —
/// which breaks the keymap's promise that every choice is answerable without
/// one.
#[test]
fn the_ability_chooser_answers_to_the_keyboard() {
    use baylee_client::Duel;
    use baylee_client::input::{ability_menu_keys, activate_card};
    use baylee_client::keys::Fired;
    use baylee_client_core::interaction::Interaction;
    use baylee_client_core::prefs::Action;

    let mut table = Table::open_with(&preset_with_a_painland());
    table.walk_to_main();

    let coast = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Yavimaya Coast")
        .expect("the painland is on the table")
        .id;

    let mut duel = Duel::default();
    duel.view = Some(table.view().clone());
    duel.interaction = Some(Interaction::new(
        table.pending.clone().expect("priority"),
        PlayerId::new(0),
    ));

    activate_card(&mut duel, coast);
    assert_eq!(
        duel.ability_menu,
        Some(coast),
        "two abilities are a menu, not a guess"
    );
    assert_eq!(duel.ability_pick, 0, "a fresh menu starts at the top");

    // The cursor walks it, and wraps rather than sticking at the end.
    assert!(ability_menu_keys(
        Fired::of_actions(&[Action::CursorDown]),
        &mut duel
    ));
    assert_eq!(duel.ability_pick, 1);
    assert!(ability_menu_keys(
        Fired::of_actions(&[Action::CursorDown]),
        &mut duel
    ));
    assert_eq!(duel.ability_pick, 0, "the list is a ring");

    // And confirm sends the entry the highlight is on — the second one,
    // which is the one a pointer used to be needed for.
    assert!(ability_menu_keys(
        Fired::of_actions(&[Action::CursorUp]),
        &mut duel
    ));
    assert_eq!(duel.ability_pick, 1);
    assert!(ability_menu_keys(
        Fired::of_actions(&[Action::Confirm]),
        &mut duel
    ));
    assert!(duel.ability_menu.is_none(), "answering closes the menu");
    assert_eq!(
        duel.outbox().first().cloned().expect("the key sent one"),
        PlayerAction::ActivateAbility {
            source: coast,
            ability_index: 1,
        }
    );
}

/// Cancel puts the menu away and sends nothing.
#[test]
fn cancelling_the_ability_chooser_activates_nothing() {
    use baylee_client::Duel;
    use baylee_client::input::{ability_menu_keys, activate_card};
    use baylee_client::keys::Fired;
    use baylee_client_core::interaction::Interaction;
    use baylee_client_core::prefs::Action;

    let mut table = Table::open_with(&preset_with_a_painland());
    table.walk_to_main();

    let coast = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Yavimaya Coast")
        .expect("the painland is on the table")
        .id;

    let mut duel = Duel::default();
    duel.view = Some(table.view().clone());
    duel.interaction = Some(Interaction::new(
        table.pending.clone().expect("priority"),
        PlayerId::new(0),
    ));
    activate_card(&mut duel, coast);
    assert!(ability_menu_keys(
        Fired::of_actions(&[Action::Cancel]),
        &mut duel
    ));
    assert!(duel.ability_menu.is_none());
    assert!(duel.outbox().is_empty(), "cancel is not an answer");
}

/// A `{T}: …` ability with no other cost fires on one tap.
///
/// The owner's words: "Ich möchte z.B. tap zum ziehen sagen, die Karte, der
/// Effekt und das wars." The line that lets it is not "mana abilities are
/// special" but the reason mana abilities were special in the first place —
/// the whole cost comes out of the card itself and the next untap step gives
/// it back.
#[test]
fn a_tap_only_ability_fires_on_one_tap() {
    use baylee_client::Duel;
    use baylee_client::abilities;
    use baylee_client::input::activate_card;
    use baylee_client_core::interaction::Interaction;

    let mut preset = preset();
    preset.seats[0].starting_battlefield.push(entry(LOREMASTER));
    let mut table = Table::open_with(&preset);
    table.walk_to_main();

    let loremaster = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name == "Sea Gate Loremaster")
        .expect("the Loremaster is on the table")
        .id;

    let interaction = Interaction::new(table.pending.clone().expect("priority"), PlayerId::new(0));
    let options = abilities::options(
        baylee_client_core::Lang::En,
        table.view(),
        &interaction,
        loremaster,
    );
    assert_eq!(options.len(), 1, "{options:?}");
    assert!(!options[0].mana, "drawing cards is not a mana ability");
    assert!(options[0].tap_only, "{{T}} alone was not read as tap-only");

    let mut duel = Duel::default();
    duel.view = Some(table.view().clone());
    duel.interaction = Some(interaction);
    activate_card(&mut duel, loremaster);
    assert!(
        duel.armed.is_none(),
        "tapping to draw asked for a confirmation"
    );
    assert_eq!(
        duel.outbox(),
        [PlayerAction::ActivateAbility {
            source: loremaster,
            ability_index: 0,
        }]
    );
}
