//! The buttons: the decision clock, combat, mulligan, keys, holds.

#[allow(clippy::wildcard_imports)] // the tests' shared fixtures
use super::*;

/// The seconds are written into the cell, and only when they change.
///
/// Two claims in one run, and the second is the one worth the harness.
/// Writing an equal `Text` still marks it changed and `bevy_text` re-lays
/// every glyph of a component it is told moved, so an unguarded writer
/// would re-shape the digits sixty times a second for the last minute of
/// every question. The guard turns that into about once a second, and
/// `Ref::is_changed` is read *in the same frame* because a change tick is
/// only visible against the run it happened in.
#[test]
fn the_seconds_are_written_in_place_and_only_when_they_move() {
    use bevy::ecs::change_detection::Ref;

    #[derive(Resource, Default)]
    struct Wrote(bool);

    fn watch(cell: Query<Ref<Text>, With<DecisionClockLabel>>, mut wrote: ResMut<Wrote>) {
        wrote.0 = cell.iter().any(|text| text.is_changed());
    }

    let mut app = App::new();
    app.init_resource::<crate::Duel>()
        .init_resource::<Wrote>()
        .insert_resource(Time::<()>::default())
        .add_systems(Update, (count_down_the_decision, watch).chain());
    let cell = app
        .world_mut()
        .spawn((DecisionClockLabel, Text::default()))
        .id();
    let says = |app: &App| app.world().entity(cell).get::<Text>().unwrap().0.clone();
    let advance = |app: &mut App, secs: f32| {
        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(std::time::Duration::from_secs_f32(secs));
        app.update();
    };

    app.world_mut()
        .resource_mut::<crate::Duel>()
        .clock
        .sync(Some(12_000), true);
    advance(&mut app, 0.0);
    assert_eq!(says(&app), "12", "the cell was never written");
    assert!(
        app.world().resource::<Wrote>().0,
        "and the write is a write"
    );

    // Two frames inside the same second: the string does not move, so
    // nothing is assigned and no glyph is re-shaped.
    advance(&mut app, 0.1);
    assert_eq!(says(&app), "12");
    assert!(
        !app.world().resource::<Wrote>().0,
        "an unchanged number was written again, which re-lays every glyph"
    );
    advance(&mut app, 0.5);
    assert!(!app.world().resource::<Wrote>().0);

    // And over the boundary it does move.
    advance(&mut app, 0.5);
    assert_eq!(says(&app), "11");
    assert!(app.world().resource::<Wrote>().0, "the second never turned");

    // A question that ends takes the number away rather than leaving the
    // last one it had standing under the next sentence.
    app.world_mut()
        .resource_mut::<crate::Duel>()
        .clock
        .sync(None, true);
    advance(&mut app, 0.0);
    assert_eq!(says(&app), "", "a countdown outlived its question");
}

/// The revision carries *whether* there is a countdown and never *what
/// it says*.
///
/// Read out of the source, the way `the_shelf_does_not_follow_the_pointer`
/// reads it, because nothing about the difference is visible at the call
/// site: a field holding the seconds would be compared and assigned like
/// any other, and would rebuild this entire tree once a second for the
/// last minute of every question — taking every `Feel` on the shelf back
/// to rest as it went. That is the whole reason the cell is written in
/// place instead.
#[test]
fn the_shelf_is_not_rebuilt_once_a_second() {
    let source = SOURCE;
    let body = source
        .split_once("pub struct LedgeRevision {")
        .expect("the struct is still called that")
        .1;
    let body = body.split_once("\n}").expect("and still closes").0;
    let clock = body
        .lines()
        .find_map(|line| line.trim().strip_prefix("pub(super) clock:"))
        .expect("the revision still carries the countdown");
    assert_eq!(
        clock.trim(),
        "bool,",
        "the revision carries the seconds themselves, so the shelf is \
         rebuilt once a second"
    );
}

/// The prompt bar stops advertising the confirm key at the moment it
/// stops working.
///
/// The other half of the combat guard. `crate::input::committed_answer`
/// refuses an empty declaration, and a row that went on drawing
/// `[Space] Attack` beside it would be a legend for a key that does
/// nothing — which is the defect this client already has elsewhere and
/// must not add one of. It is also the legend that *taught* the mistake:
/// with nothing declared, "Attack" and "None" did the same thing under
/// two names, and the one wearing the key was the one a player was
/// already pressing.
///
/// Both directions, because a row that offered the confirm answer to
/// nobody would pass a one-sided version of this.
#[test]
fn the_combat_row_offers_the_commit_only_once_something_is_declared() {
    use baylee_core::ids::Defender;
    use baylee_engine::choice::Pending;

    let question = || Pending::ChooseAttackers {
        player: baylee_core::ids::PlayerId::new(0),
        attackers: vec![baylee_core::ids::ObjectId::new(3, 0)],
        defenders: vec![Defender::Player(baylee_core::ids::PlayerId::new(1))],
        required: Vec::new(),
        limits: Vec::new(),
    };
    let says = |duel: &Duel| -> Vec<Says> {
        answers_for(duel, Lang::En, false, false, false)
            .into_iter()
            .map(|(says, _)| says)
            .collect()
    };

    let mut duel = Duel {
        interaction: Some(baylee_client_core::interaction::Interaction::new(
            question(),
            baylee_core::ids::PlayerId::new(0),
        )),
        ..Duel::default()
    };
    assert_eq!(
        says(&duel),
        vec![
            Says::Answer(PromptAction::AimNext),
            Says::Answer(PromptAction::DeclareNothing),
        ],
        "with nothing declared the bar offers aiming and declining, and no key for a third"
    );

    assert!(
        duel.interaction
            .as_mut()
            .is_some_and(|i| i.declare_attacker(
                baylee_core::ids::ObjectId::new(3, 0),
                Defender::Player(baylee_core::ids::PlayerId::new(1))
            )),
        "the candidate the question offered is declarable"
    );
    assert_eq!(
        says(&duel),
        vec![
            Says::Answer(PromptAction::AimNext),
            Says::Answer(PromptAction::Confirm),
            Says::Answer(PromptAction::DeclareNothing),
        ],
        "a declaration standing is what the commit button is for"
    );
}

/// The mulligan row offers a further mulligan only while the question
/// allows one (CR 103.5, `Pending::Mulligan::can_take`).
#[test]
fn the_mulligan_button_goes_when_the_question_takes_no_further_one() {
    use baylee_core::ids::PlayerId;
    use baylee_engine::choice::Pending;
    let me = PlayerId::new(0);
    for (can_take, offered) in [
        (true, vec![PromptAction::Keep, PromptAction::Mulligan]),
        (false, vec![PromptAction::Keep]),
    ] {
        let duel = Duel {
            interaction: Some(baylee_client_core::interaction::Interaction::new(
                Pending::Mulligan {
                    player: me,
                    taken: 8,
                    next_is_free: false,
                    can_take,
                },
                me,
            )),
            ..Duel::default()
        };
        let answers: Vec<PromptAction> = answers_for(&duel, Lang::En, false, false, false)
            .into_iter()
            .filter_map(|(says, _)| match says {
                Says::Answer(action) => Some(action),
                Says::Command(_) => None,
            })
            .collect();
        assert_eq!(answers, offered, "can_take {can_take}");
    }
}

/// The countdown stands in the button the clock presses, wherever the
/// row has that button, and beside the question everywhere else (#258).
#[test]
fn the_countdown_stands_in_the_button_the_clock_presses() {
    use baylee_core::ids::{Defender, ObjectId, PlayerId};
    use baylee_engine::choice::{LegalActions, Pending, YesNoPrompt};
    let me = PlayerId::new(0);
    let asked = |pending: Pending| Duel {
        interaction: Some(baylee_client_core::interaction::Interaction::new(
            pending, me,
        )),
        ..Duel::default()
    };
    let placed = |duel: &Duel, shown: bool, armed: bool| {
        let answers = answers_for(duel, Lang::En, false, false, false);
        clock_placement(duel, shown, armed, &answers)
    };
    let yes_no = |prompt| Pending::YesNo {
        player: me,
        prompt,
        source: None,
    };

    let priority = asked(Pending::Priority {
        player: me,
        legal: Box::new(LegalActions::default()),
    });
    assert_eq!(
        placed(&priority, true, false),
        (Clock::InButton, Some(PromptAction::Confirm)),
        "at priority the clock passes, and the Pass button says so"
    );
    for (pending, button) in [
        (
            Pending::Mulligan {
                player: me,
                taken: 0,
                next_is_free: true,
                can_take: true,
            },
            PromptAction::Keep,
        ),
        (
            Pending::ChooseAttackers {
                player: me,
                attackers: vec![ObjectId::new(3, 0)],
                defenders: vec![Defender::Player(PlayerId::new(1))],
                required: Vec::new(),
                limits: Vec::new(),
            },
            PromptAction::DeclareNothing,
        ),
        (yes_no(YesNoPrompt::Kicker), PromptAction::No),
    ] {
        assert_eq!(
            placed(&asked(pending.clone()), true, false),
            (Clock::InButton, Some(button)),
            "{pending:?}"
        );
    }

    // The house answers these, so no button can carry its seconds.
    for pending in [
        yes_no(YesNoPrompt::CommanderZone {
            card: ObjectId::new(3, 0),
        }),
        Pending::DiscardChoice {
            player: me,
            count: 1,
        },
    ] {
        assert_eq!(
            placed(&asked(pending.clone()), true, false),
            (Clock::Beside, None),
            "{pending:?}"
        );
    }
    // An armed deed takes the row away, and no clock means no cell.
    assert_eq!(placed(&priority, true, true), (Clock::Beside, None));
    assert_eq!(placed(&priority, false, false), (Clock::None, None));
    // Another seat's question puts no answers on this shelf.
    let theirs = asked(Pending::Priority {
        player: PlayerId::new(1),
        legal: Box::new(LegalActions::default()),
    });
    let answers = answers_for(&theirs, Lang::En, false, true, false);
    assert_eq!(
        clock_placement(&theirs, true, false, &answers),
        (Clock::Beside, None)
    );
}

/// A command's cap names the key that does the same thing, and it comes
/// out of the keymap like every other cap on the shelf.
///
/// The bridge [`keys_for`] uses for an answer —
/// `baylee_client_core::ledge::shortcut_for` — reaches `PromptAction`
/// alone, so a [`Says::Command`] names its action in the renderer. This is
/// what makes that a mapping rather than a guess: unbind
/// `Action::HoldForStack` in the default keymap and the cap goes, which is
/// right; point it at another action and this fails, which is the part
/// worth having.
#[test]
fn a_command_wears_the_key_that_does_the_same_thing() {
    let prefs = crate::prefs::Prefs::default();
    let row = vec![
        (Says::Answer(PromptAction::Confirm), "Pass".to_string()),
        (
            Says::Command(super::MenuAction::HoldForStack),
            "Resolve the stack".to_string(),
        ),
    ];
    let caps = keys_for(&prefs, &row, false, false);
    assert_eq!(
        caps.get(1).and_then(Option::as_deref),
        Some("F6"),
        "the whole claim of the cap is that this key does this: {caps:?}"
    );
    // The counter-half: a command with no key of its own wears none,
    // rather than borrowing the one beside it.
    let row = vec![(
        Says::Command(super::MenuAction::Concede),
        "Concede".to_string(),
    )];
    assert_eq!(
        keys_for(&prefs, &row, false, false),
        vec![None],
        "a concession has no key and must not grow one here"
    );
}

/// The way out of a hold wears the key that ends a hold, and the way out
/// of the autopilot wears nothing.
///
/// One button and one sentence for two mechanisms (§4.4), which is right
/// — a player who has stopped being asked does not care which of them did
/// it — and the keycap is the one place the difference is real. `F6`
/// cancels a running engine hold; **no** key ends the autopilot, so a cap
/// on that button would promise a way out that the keyboard does not
/// have.
#[test]
fn the_way_out_of_a_hold_wears_a_key_and_the_way_out_of_the_pilot_does_not() {
    let prefs = crate::prefs::Prefs::default();
    let row = vec![(
        Says::Command(super::MenuAction::ReleaseHold),
        "Ask me again".to_string(),
    )];
    assert_eq!(
        keys_for(&prefs, &row, false, true),
        vec![Some("F6".to_string())],
        "a running hold is cancelled by F6, so the button says F6"
    );
    assert_eq!(
        keys_for(&prefs, &row, false, false),
        vec![None],
        "no key ends the autopilot, so the same button wears no cap"
    );
}
