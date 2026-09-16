//! Arming: a deed that is built and held rather than sent.
//!
//! A choice made here does not reach the engine. It arms the row, and the
//! same key pressed again is what sends it — which is the whole reason the
//! client has an outbox at all.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// Picking from the ability sheet *arms*; it does not send, and the sheet
/// stays open while it is armed.
///
/// The sheet disambiguates and confirming is a separate statement, which is
/// the whole of arm-then-act. The painland test above cannot show it: both of
/// Yavimaya Coast's abilities make mana, and a mana ability is the one
/// exemption, so that path goes straight onto the wire. A planeswalker is the
/// other shape — two abilities, neither of them mana — and it is the one this
/// branch actually runs in.
///
/// Staying open is the half the sheet added. The row's keycap goes gilt and
/// the footer says to press the same digit again, so a sheet that closed on
/// the arming would take away the only key it had just named — and the two
/// states have two ways out, the first escape disarming and the second
/// closing.
#[test]
fn picking_from_the_chooser_arms_rather_than_sends() {
    use baylee_client::input::{ability_menu_keys, activate_card, armed_keys};
    use baylee_client::keys::Fired;
    use baylee_client::{Deed, Duel};
    use baylee_client_core::interaction::Interaction;
    use baylee_client_core::prefs::Action;

    let mut table = Table::open_with(&preset_with_a_planeswalker());
    table.walk_to_main();

    let karn = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name.starts_with("Karn"))
        .expect("the planeswalker is on the table")
        .id;

    let mut duel = Duel::default();
    duel.view = Some(table.view().clone());
    duel.interaction = Some(Interaction::new(
        table.pending.clone().expect("priority"),
        PlayerId::new(0),
    ));

    let options = baylee_client::abilities::options(
        baylee_client_core::Lang::En,
        duel.view.as_ref().expect("a view"),
        duel.interaction.as_ref().expect("a choice"),
        karn,
    );
    assert!(
        options.len() >= 2,
        "a loyalty ability is not a mana ability, so both are offered: {options:?}"
    );
    assert!(
        options.iter().all(|o| !o.mana),
        "none of a planeswalker's abilities makes mana"
    );

    activate_card(&mut duel, karn);
    assert_eq!(duel.ability_menu, Some(karn), "two abilities are a menu");

    assert!(ability_menu_keys(
        Fired::of_actions(&[Action::CursorDown]),
        &mut duel
    ));
    assert!(ability_menu_keys(
        Fired::of_actions(&[Action::Confirm]),
        &mut duel
    ));
    assert_eq!(
        duel.ability_menu,
        Some(karn),
        "the sheet stays open while a row is armed: the digit that armed it \
         is the digit that sends it, and a sheet that closed here would take \
         that digit away"
    );
    assert!(
        duel.outbox().is_empty(),
        "picking is not confirming — nothing is on the wire yet"
    );
    assert_eq!(
        duel.armed.as_ref().map(|a| a.deed.clone()),
        Some(Deed::Ability(options[1].action.clone())),
        "what is armed is the entry the highlight was on"
    );

    // Escape takes the arming back and leaves the sheet standing, because
    // those are two states and there are two ways out of them.
    assert!(armed_keys(Fired::of_actions(&[Action::Cancel]), &mut duel));
    assert!(duel.armed.is_none(), "the first escape disarms");
    assert_eq!(duel.ability_menu, Some(karn), "and leaves the sheet open");
    assert!(ability_menu_keys(
        Fired::of_actions(&[Action::Cancel]),
        &mut duel
    ));
    assert!(duel.ability_menu.is_none(), "the second closes it");

    // Arm it again, and the confirm after it is the send.
    duel.ability_menu = Some(karn);
    duel.ability_pick = 1;
    assert!(ability_menu_keys(
        Fired::of_actions(&[Action::Confirm]),
        &mut duel
    ));
    assert!(armed_keys(Fired::of_actions(&[Action::Confirm]), &mut duel));
    assert_eq!(
        duel.outbox().first().cloned().expect("the key sent one"),
        options[1].action,
    );
    assert!(duel.armed.is_none(), "sending disarms");
    assert!(
        duel.ability_menu.is_none(),
        "and the sheet has been answered"
    );
}

/// The digit drawn on a row arms that row, and the same digit sends it.
///
/// This is the two-stage mechanic reached the way a player reaches it, and
/// the three things it has bitten on before are all here: the arming has to
/// *move* when a second row is pressed rather than adding a second armed
/// deed, the send has to go through `fire_armed` so the engine's current
/// offer is what is sent, and the sheet has to stay open in between —
/// because the digit that armed the row is the digit that sends it, and a
/// sheet that closed on the arming would take that digit away.
#[test]
fn a_digit_arms_the_row_it_is_drawn_on_and_the_same_digit_sends_it() {
    use baylee_client::input::{activate_card, sheet_digit};
    use baylee_client::{Deed, Duel};
    use baylee_client_core::interaction::Interaction;

    let mut table = Table::open_with(&preset_with_a_planeswalker());
    table.walk_to_main();

    let karn = table
        .view()
        .battlefield
        .iter()
        .find(|o| o.name.starts_with("Karn"))
        .expect("the planeswalker is on the table")
        .id;

    let mut duel = Duel::default();
    duel.view = Some(table.view().clone());
    duel.interaction = Some(Interaction::new(
        table.pending.clone().expect("priority"),
        PlayerId::new(0),
    ));

    let options = baylee_client::abilities::options(
        baylee_client_core::Lang::En,
        duel.view.as_ref().expect("a view"),
        duel.interaction.as_ref().expect("a choice"),
        karn,
    );
    assert!(options.len() >= 2, "two rows to tell apart: {options:?}");
    assert!(
        options.len() <= baylee_client_core::abilitysheet::PAGE,
        "one page, so `0` is a key the sheet never drew: {options:?}"
    );

    activate_card(&mut duel, karn);
    assert_eq!(duel.ability_menu, Some(karn), "two abilities are a sheet");

    assert!(sheet_digit(&mut duel, '2'), "the second row wears a `2`");
    assert_eq!(duel.ability_pick, 1, "and the cursor goes to it");
    assert_eq!(
        duel.armed.as_ref().map(|a| a.deed.clone()),
        Some(Deed::Ability(options[1].action.clone())),
        "a loyalty ability costs something, so the digit arms it"
    );
    assert_eq!(duel.ability_menu, Some(karn), "the sheet stays open");
    assert!(duel.outbox().is_empty(), "nothing is on the wire yet");

    // A different digit moves the arming rather than adding to it: there is
    // one `Duel::armed`, and a player changing their mind is the ordinary
    // case rather than an error.
    assert!(sheet_digit(&mut duel, '1'), "the first row wears a `1`");
    assert_eq!(
        duel.armed.as_ref().map(|a| a.deed.clone()),
        Some(Deed::Ability(options[0].action.clone())),
        "the arming moved"
    );
    assert!(duel.outbox().is_empty(), "and still nothing was sent");

    // A key nothing drew. The pager is the tenth row and this sheet has two.
    let before = (duel.ability_page, duel.ability_pick);
    assert!(
        !sheet_digit(&mut duel, '0'),
        "`0` is the pager and there is no second page"
    );
    assert_eq!(
        (duel.ability_page, duel.ability_pick),
        before,
        "so it moved nothing"
    );
    assert!(duel.outbox().is_empty(), "and sent nothing");

    // The same digit again is the send.
    assert!(
        sheet_digit(&mut duel, '1'),
        "the row is pressed a second time"
    );
    assert_eq!(
        duel.outbox().first().cloned().expect("the digit sent one"),
        options[0].action,
    );
    assert!(duel.armed.is_none(), "sending disarms");
    assert!(
        duel.ability_menu.is_none(),
        "and the sheet has been answered"
    );
}

/// An armed run can come back a different answer, and then it is a cast.
///
/// This is the one arming path that re-resolves to something *other* than
/// itself. Between the two taps this seat holds priority, so the board can
/// only have been changed by this seat — and the one thing it can have done
/// is tap a land by hand. Once that pays the cost the engine offers the spell
/// outright, and a run started anyway would tap two more lands and float mana
/// nobody asked for.
///
/// The ordering is what the test is really about: `play_card` is read
/// *before* `reachable`, so `duel.reachable` is deliberately left saying yes
/// here. What the client offered to do a moment ago must not outvote what the
/// engine is offering now.
#[test]
fn an_armed_run_becomes_a_plain_cast_when_the_lands_are_tapped_by_hand() {
    use baylee_client::input::{activate_card, armed_keys};
    use baylee_client::keys::Fired;
    use baylee_client::{Deed, Duel};
    use baylee_client_core::interaction::Interaction;
    use baylee_client_core::prefs::Action;

    let mut table = Table::open();
    table.walk_to_main();

    let spell = table
        .view()
        .hand
        .iter()
        .find(|c| c.name == "Great Divide Guide")
        .expect("the creature is in the opening hand")
        .id;

    let mut duel = Duel::default();
    duel.view = Some(table.view().clone());
    duel.interaction = Some(Interaction::new(
        table.pending.clone().expect("priority"),
        PlayerId::new(0),
    ));
    duel.reachable = std::iter::once(spell).collect();

    activate_card(&mut duel, spell);
    let Some(Deed::Run { plan, .. }) = duel.armed.as_ref().map(|a| a.deed.clone()) else {
        panic!("a spell the lands can pay for arms a run: {:?}", duel.armed);
    };
    assert!(duel.outbox().is_empty(), "arming puts nothing on the wire");

    // The player pays it themselves instead, which is the thing that was
    // always allowed and is the only way the board can move here.
    for step in &plan.steps {
        table.submit(PlayerAction::ActivateManaAbility {
            source: step.source,
        });
    }
    assert!(
        table.legal().castable.contains(&spell),
        "with the mana floating the engine offers the spell"
    );
    duel.view = Some(table.view().clone());
    duel.interaction = Some(Interaction::new(
        table.pending.clone().expect("priority"),
        PlayerId::new(0),
    ));

    // Confirm now casts. No run is started, and no further land is tapped.
    assert!(armed_keys(Fired::of_actions(&[Action::Confirm]), &mut duel));
    assert_eq!(
        duel.outbox().first().cloned().expect("the key sent one"),
        PlayerAction::CastSpell { card: spell },
        "the armed run resolved to the cast it was standing in for"
    );
    assert!(duel.mana_run.is_none(), "nothing was tapped for it");
    assert!(duel.armed.is_none(), "sending disarms");
}
