use super::*;
use baylee_client_core::test_support::ViewBuilder;
use baylee_view::SeatIdentity;

#[test]
fn large_life_totals_fit_beside_the_heart_without_losing_digits() {
    let font = swash::FontRef::from_index(
        include_bytes!("../../../assets/fonts/AlegreyaSans-Bold.ttf"),
        0,
    )
    .unwrap();
    let metrics = font.glyph_metrics(&[]).scale(1.0);
    for ch in "0123456789-".chars() {
        assert!(metrics.advance_width(font.charmap().map(ch)) <= 0.6);
    }
    for width in [54.0, 72.0, 108.0] {
        for value in [i32::MIN, -25476, -1, 0, 40, 999, 25476, i32::MAX] {
            let size = life_number_size(value, 24.0, width, 18.0);
            let ink: f32 = value
                .to_string()
                .chars()
                .map(|ch| metrics.advance_width(font.charmap().map(ch)))
                .sum::<f32>()
                * size
                * UI_SCALE;
            assert!(ink + 18.0 + 3.0 <= width, "{value} overflows {width}");
        }
    }
    assert_eq!(life_number_size(40, 24.0, 108.0, 18.0), 24.0);
    // Five digits still draw at least as large as before the UI scale
    // increased, even when the nominal size must shrink to fit.
    assert!(life_number_size(25476, 24.0, 108.0, 18.0) * UI_SCALE >= 28.8);
}

/// What one name cell is asked about, handed to the system that draws it.
///
/// A resource and a one-system app rather than a direct call, because
/// `name` builds through `Commands` and what is under test is the tree it
/// leaves behind — the same reason `combatlines::running` and the ledge's
/// harness are apps rather than arithmetic.
#[derive(Resource, Clone)]
struct Asked {
    lang: Lang,
    role: SeatRole,
    statics: GameStatic,
    view: PlayerView,
}

fn draw(mut commands: Commands, asked: Res<Asked>, fonts: Res<UiFonts>) {
    let seat = &asked.view.seats[OTHER];
    name(
        &mut commands,
        asked.lang,
        &asked.view,
        Some(&asked.statics),
        seat,
        asked.role,
        &fonts,
        240.0,
        22.0,
    );
}

/// The chair every test below is about: the opponent's, not the viewer's.
///
/// Not seat 0, because `own_seat_name` wraps the viewing seat's own name
/// — "You (Alice)" — and a test reading that would be reading the wrapper
/// rather than the name. Nothing about a role differs between the two
/// chairs; the wrapper is simply noise here.
const OTHER: usize = 1;

/// One name cell, drawn for a chair in `role` and called `called`.
fn cell(role: SeatRole, lang: Lang, called: &str) -> App {
    let view = ViewBuilder::new(2).build();
    let mut statics = baylee_client_core::test_support::statics(1);
    statics.seats = vec![SeatIdentity {
        player: view.seats[OTHER].player,
        display_name: called.to_string(),
        is_ai: role == SeatRole::House,
        away: role == SeatRole::Away,
        team: None,
    }];
    let mut app = App::new();
    app.insert_resource(Asked {
        lang,
        role,
        statics,
        view,
    })
    .insert_resource(fonts())
    .add_systems(Update, draw);
    app.update();
    app
}

/// A two-seat table with its shelves already measured, running the real
/// [`sync_seat_bars`].
///
/// The shelves are handed in rather than projected, because what is under
/// test is the gate and not the camera: `measure_shelves` runs off a rig
/// and a window, and a harness that needed both would be testing those.
/// Long enough for `Density::Full`, so both seats get a bar with a name on
/// it. `Cloth` and its materials are left out on purpose — that is the
/// branch a machine with no GPU takes.
fn table() -> App {
    let mut duel = Duel {
        view: Some(ViewBuilder::new(2).build()),
        statics: Some(baylee_client_core::test_support::statics(1)),
        ..Duel::default()
    };
    if let Some(statics) = duel.statics.as_mut() {
        statics.seats = (0..2)
            .map(|at| SeatIdentity {
                player: PlayerId::new(at),
                display_name: format!("Seat {at}"),
                is_ai: false,
                away: false,
                team: None,
            })
            .collect();
    }
    crate::rebuild_board(&mut duel);
    let shelves = Shelves(
        (0..2)
            .map(|at| {
                (
                    PlayerId::new(at),
                    Shelf {
                        middle: Vec2::new(600.0, 200.0 + f32::from(at) * 400.0),
                        along: 1100.0,
                        depth: 46.0,
                        tilt: 0.0,
                        density: Density::Full,
                    },
                )
            })
            .collect(),
    );
    let mut app = App::new();
    app.insert_resource(duel)
        .insert_resource(shelves)
        .insert_resource(fonts())
        .insert_resource(crate::settings::ClientSettings::default())
        .init_resource::<crate::prefs::Prefs>()
        .init_resource::<BarRevision>()
        .add_systems(Update, sync_seat_bars);
    app.update();
    app
}

/// Fonts with no asset server behind them: what is under test is the tree
/// the bar builds, and none of that is the GPU's.
fn fonts() -> UiFonts {
    UiFonts {
        text: Handle::default(),
        medium: Handle::default(),
        bold: Handle::default(),
        italic: Handle::default(),
        medium_italic: Handle::default(),
        serif: Handle::default(),
        serif_italic: Handle::default(),
        icons: Handle::default(),
        mana: Handle::default(),
    }
}

/// Every word the cell put on the screen.
fn said(app: &mut App) -> Vec<String> {
    let mut q = app.world_mut().query::<&Text>();
    q.iter(app.world()).map(|t| t.0.clone()).collect()
}

/// The clock, which is the whole of what a held chair says.
const CLOCK: &str = "\u{f017}";

/// A chair the house is *holding* wears a clock; the other two do not.
///
/// Three cases and not one, because each of the other two is a different
/// way to get this wrong. A present player wearing a clock is the table
/// telling everyone somebody has left when nobody has. And a chair the
/// house plays by arrangement wearing one would be saying the same thing
/// twice — its name already says the house is there — while spending the
/// name's own room on the repetition.
#[test]
fn only_a_held_chair_wears_the_clock() {
    let mut held = cell(SeatRole::Away, Lang::En, "Alice");
    assert!(
        said(&mut held).iter().any(|word| word == CLOCK),
        "a chair the house is holding says so beside its name: {:?}",
        said(&mut held)
    );
    for role in [SeatRole::Present, SeatRole::House] {
        let mut other = cell(role, Lang::En, "Alice");
        assert!(
            !said(&mut other).iter().any(|word| word == CLOCK),
            "{role:?} is not an interruption and must not wear its mark: \
                 {:?}",
            said(&mut other)
        );
    }
}

/// Before turn 1 every seat still deciding its opening hand wears the
/// caret (#257), not only the one this view's `awaiting` names — which
/// is this seat until it keeps and nobody after. The caret asks the mat
/// rim's own predicate, so this is also the proof that it does.
#[test]
fn every_seat_still_deciding_its_hand_wears_the_caret() {
    fn draw_carets(mut commands: Commands, asked: Res<Asked>, fonts: Res<UiFonts>) {
        for seat in &asked.view.seats {
            caret(&mut commands, &asked.view, seat, &fonts, 12.0, 22.0);
        }
    }
    // Seat 0's view, as `awaiting` and `deciding` would have it.
    let lit = |awaiting: Option<u8>, deciding: &[u8]| -> Vec<u8> {
        let mut view = ViewBuilder::new(2).build();
        view.awaiting = awaiting.map(PlayerId::new);
        view.deciding = deciding.iter().copied().map(PlayerId::new).collect();
        let mut app = App::new();
        app.insert_resource(Asked {
            lang: Lang::En,
            role: SeatRole::Present,
            statics: baylee_client_core::test_support::statics(1),
            view,
        })
        .insert_resource(fonts())
        .add_systems(Update, draw_carets);
        app.update();
        let world = app.world_mut();
        let carets: Vec<(PlayerId, Vec<Entity>)> = world
            .query::<(&SeatInk, &Children)>()
            .iter(world)
            .map(|(ink, kids)| (ink.player, kids.to_vec()))
            .collect();
        assert_eq!(carets.len(), 2, "one caret per seat");
        let mut lit: Vec<u8> = carets
            .into_iter()
            .filter(|(_, kids)| {
                kids.iter().any(|kid| {
                    world
                        .get::<TextColor>(*kid)
                        .is_some_and(|c| c.0 != Color::NONE)
                })
            })
            .map(|(player, _)| player.get())
            .collect();
        lit.sort_unstable();
        lit
    };
    assert_eq!(lit(Some(0), &[0, 1]), [0, 1], "nobody has kept yet");
    assert_eq!(
        lit(None, &[1]),
        [1],
        "seat 0 has kept and its view names nobody, but seat 1 is still deciding"
    );
    assert_eq!(lit(Some(1), &[]), [1], "turn 1: the one seat asked");
}

/// A chair the house plays is called the house in the player's language,
/// and a chair it is merely holding keeps its player's name.
///
/// The second half is the one worth having. `away` and `is_ai` are two
/// fields rather than one precisely so that a thirty-second hiccup does
/// not rename somebody's chair to the house — a rename that would still
/// be on the bar after they came back.
#[test]
fn a_house_chair_is_called_the_house_in_the_players_own_language() {
    let mut house = cell(SeatRole::House, Lang::De, "House AI");
    assert!(
        said(&mut house).iter().any(|word| word == "Haus-KI"),
        "a German player was still being shown an English word: {:?}",
        said(&mut house)
    );

    let mut named = cell(SeatRole::House, Lang::De, "Solide 1");
    assert!(said(&mut named).iter().any(|word| word == "Solide 1"));
    assert!(!said(&mut named).iter().any(|word| word == "Haus-KI"));

    let mut held = cell(SeatRole::Away, Lang::De, "Alice");
    assert!(
        said(&mut held).iter().any(|word| word == "Alice"),
        "a held chair still belongs to the player who left it: {:?}",
        said(&mut held)
    );
    assert!(
        !said(&mut held).iter().any(|word| word == "Haus-KI"),
        "and must not be relabelled the house while they are gone"
    );
}

/// A chair changing hands redraws the bars, although no view arrived.
///
/// The one field in [`BarRevision`] that does not come from the view, and
/// the reason it has to be there. A roster is its own payload: the host
/// marks every seat's roster stale when a chair changes hands and the
/// fresh `GameStatic` reaches this client as a separate message, so a bar
/// gated on the view's `seq` alone is built from whichever roster was in
/// hand at the time and goes on saying so.
///
/// Both directions, and the **second** is the one worth having: a clock
/// that arrives late is a nuisance, and a clock that stays after the
/// player has come back is the bar telling the table something untrue —
/// against the whole design of `away`, which is that a thirty-second
/// hiccup leaves no trace.
///
/// The counter-half is the third `update`: with the roster settled and
/// nothing else moved, the bars are *not* rebuilt. Without it this test
/// would pass just as well against a gate that had been deleted.
#[test]
fn a_chair_changing_hands_redraws_the_bar_and_settling_does_not() {
    let bars = |app: &mut App| {
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, With<SeatBarRoot>>();
        q.iter(app.world()).next()
    };
    let set_away = |app: &mut App, away: bool| {
        let mut duel = app.world_mut().resource_mut::<Duel>();
        if let Some(statics) = duel.statics.as_mut() {
            statics.seats[OTHER].away = away;
        }
        crate::rebuild_board(&mut duel);
    };

    let mut app = table();
    let first = bars(&mut app).expect("the seats were given bars");

    // The player steps away. No view arrives — only the roster changes.
    set_away(&mut app, true);
    app.update();
    let held = bars(&mut app).expect("the seats still have bars");
    assert_ne!(
        held, first,
        "the roster said a chair had changed hands and the bar did not              notice, so its clock is one view behind"
    );

    // Nothing moves. The gate is a gate.
    app.update();
    assert_eq!(
        bars(&mut app),
        Some(held),
        "nothing changed and the bars were rebuilt anyway, so the test              above is not measuring a gate at all"
    );

    // And the player comes back.
    set_away(&mut app, false);
    app.update();
    let back = bars(&mut app).expect("and after they return");
    assert_ne!(
        back, held,
        "the clock would still be on a bar whose player is answering              again, which is the half of `away` that must leave no trace"
    );
}

/// The mark does not give way; the name does.
///
/// This is the claim that decides whether any of the above is ever *seen*,
/// and it is asserted on the two nodes rather than on a measured layout
/// because nothing in this repository runs `bevy_ui`'s layout in a test —
/// every other `ComputedNode` here is hand-written by the test that reads
/// it. What the pair stands for: a flex item's `min-width` is `auto`,
/// which for a `NoWrap` text resolves to the whole string, so a name long
/// enough to fill the cell refuses to shrink and everything after it is
/// pushed past the `clip_x` edge. The mark has been in this tree since the
/// bar was written and was invisible for exactly the names that matter.
#[test]
fn a_long_name_gives_way_to_the_mark_and_not_the_other_way_round() {
    let mut app = cell(SeatRole::Away, Lang::En, "Alexandra Wintersmith-Ó");
    let mut q = app.world_mut().query::<(&Text, &Node)>();
    let nodes: Vec<(String, Val, f32)> = q
        .iter(app.world())
        .map(|(text, node)| (text.0.clone(), node.min_width, node.flex_shrink))
        .collect();
    let name = nodes
        .iter()
        .find(|(word, _, _)| word != CLOCK)
        .expect("the cell drew a name");
    let mark = nodes
        .iter()
        .find(|(word, _, _)| word == CLOCK)
        .expect("and a clock beside it");
    assert_eq!(
        name.1,
        px(0),
        "the name keeps `min-width: auto`, so it is the whole string wide \
             and will not shrink: {nodes:?}"
    );
    assert!(
        name.2 > 0.0,
        "and it has to be the one that gives way: {nodes:?}"
    );
    // Not positive, rather than exactly zero: what the row does is divide
    // the overflow by the shrink factors, so any factor at all is a share
    // of the shortfall and the claim is that the mark takes none of it.
    assert!(
        mark.2 <= 0.0,
        "the mark must not be the thing that shrinks — it is one glyph, \
             and a shrunk glyph is no glyph: {nodes:?}"
    );
}
