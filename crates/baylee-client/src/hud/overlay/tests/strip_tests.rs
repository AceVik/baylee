//! The owed and players strips on the shelf.

#[allow(clippy::wildcard_imports)] // the tests' shared fixtures
use super::*;

/// A seat with nothing to answer: the opponent holds priority.
///
/// The state the mana pool used to vanish in, and the only one in which
/// the shelf's rule and the chip's rule differ.
fn duel_watching() -> Duel {
    let mut duel = Duel {
        interaction: Some(baylee_client_core::Interaction::new(
            baylee_engine::choice::Pending::Priority {
                player: PlayerId::new(1),
                legal: Box::new(baylee_engine::choice::LegalActions::default()),
            },
            PlayerId::new(0),
        )),
        ..Duel::default()
    };
    duel.view = Some(baylee_client_core::test_support::ViewBuilder::new(2).build());
    crate::rebuild_board(&mut duel);
    assert!(!duel.is_my_turn_to_act(), "this seat is watching");
    duel
}

/// The strip on the shelf's right end stands only while this seat owes
/// in a payment window of its own (CR 605.3a), and floating mana alone
/// raises nothing there: since 08.10.2026 the pool is on each seat's plate
/// on the table (`seatbar::attached`), and the old separate pool is gone.
#[test]
fn the_owed_strip_stands_only_while_this_seat_owes() {
    let mut watching = bar_of(pool_of(3, "three"));
    assert!(
        !strip_is_shown(&mut watching),
        "mana floats, nothing is owed: there is no strip to read"
    );
    assert!(
        !said(&mut watching).contains(&Phrase::ManaPool.text(Lang::En).to_string()),
        "the pool's own label is gone with it"
    );

    let mut owing = bar_of(owing(true));
    assert!(
        strip_is_shown(&mut owing),
        "a payment window: the strip is up"
    );
    assert!(
        said(&mut owing).contains(&Phrase::Owed.text(Lang::En).to_string()),
        "and it says what is owed: {:?}",
        said(&mut owing)
    );
}

/// This seat's own payment window ({2}{G} owed), or the same seat with
/// nothing owed.
fn owing(open: bool) -> Duel {
    let cost = baylee_core::mana::ManaCost::try_parse("{2}{G}").expect("a cost");
    crate::owed_tests::seat_with_two_forests(open.then_some(cost))
}

/// Whether the owed strip is drawn at all.
fn strip_is_shown(app: &mut App) -> bool {
    let mut q = app
        .world_mut()
        .query_filtered::<&Visibility, With<ledge::pool::PoolStrip>>();
    q.iter(app.world()).any(|seen| *seen != Visibility::Hidden)
}

/// The shelf stands off the table above it and off the hand below it, and
/// it leans on the hand the more lightly of the two.
///
/// The owner asked for the second cast on 14.09.2026 — "zum Tisch hin als
/// auch zur Hand hin (zur Hand etwas leichter)". It is read off the
/// **built nodes** and not off the constants, because the constants being
/// right is not the claim: one cast written twice would satisfy every
/// number in `ledge.rs` and still be a lid.
///
/// They stopped being a `BoxShadow` when the shelf stopped being opaque,
/// and the second half of this test is why: a `BoxShadow` is the node's
/// own rectangle offset and blurred, so both casts lay most of their
/// weight *inside* the shelf — invisible behind an opaque one, and about
/// two thirds black across a translucent one. So each cast has to fall
/// wholly outside the shelf's box, and that is asserted rather than
/// assumed.
#[test]
fn the_shelf_casts_both_ways_and_more_softly_onto_the_hand() {
    let mut app = bar_of(duel_with(false));
    let shelf = app
        .world_mut()
        .query_filtered::<Entity, With<ledge::LedgeShelf>>()
        .iter(app.world())
        .next()
        .expect("the shelf is built");
    let children: Vec<Entity> = app
        .world()
        .entity(shelf)
        .get::<Children>()
        .expect("the shelf has children")
        .iter()
        .collect();
    let px = |v: Val| match v {
        Val::Px(p) => p,
        other => panic!("a cast is measured in pixels, not {other:?}"),
    };
    let casts: Vec<(f32, f32, f32)> = children
        .into_iter()
        .filter_map(|child| {
            // A cast is the only child of the shelf that is a gradient;
            // the three columns are laid out and carry no paint at all.
            let gradient = app.world().entity(child).get::<BackgroundGradient>()?;
            let node = app.world().entity(child).get::<Node>()?;
            let (top, height) = (px(node.top), px(node.height));
            let weight = gradient.0.iter().fold(0.0_f32, |most, g| match g {
                Gradient::Linear(l) => l
                    .stops
                    .iter()
                    .fold(most, |m, stop| m.max(stop.color.alpha())),
                _ => most,
            });
            Some((top, height, weight))
        })
        .collect();
    let up = casts
        .iter()
        .find(|(top, _, _)| *top < 0.0)
        .expect("nothing is cast onto the table");
    let down = casts
        .iter()
        .find(|(top, _, _)| *top > 0.0)
        .expect("nothing is cast onto the hand");
    assert!(
        down.2 < up.2,
        "the hand's side is the lighter one: {} against {}",
        down.2,
        up.2
    );
    assert!(down.2 > 0.0, "and it is still a cast, not an absence");
    // Wholly outside the shelf, both of them: the one above ends where
    // the shelf begins, the one below begins where the shelf ends.
    //
    // **Both insets are read from the shelf's padding box**, which is
    // where this assertion used to be wrong in exactly the way the code
    // was. An absolutely-positioned child's `top` is measured from its
    // containing block's padding box, and the shelf carries
    // `border: UiRect::top(px(LIP))` — so `top: -LIFT_UP_H` put the
    // gradient's darkest end one pixel *inside* the bar, on the one line
    // the cloth paints, and `up.0 + up.1 <= 0.0` was satisfied by the
    // overlap rather than in spite of it. Converting to the border box is
    // one addition, and it is the whole of what the claim is about.
    let lip = crate::hud::LEDGE_LIP;
    assert!(
        up.0 + up.1 + lip <= 0.0,
        "the table's cast reaches {} pixels into the shelf",
        up.0 + up.1 + lip
    );
    assert!(
        down.0 + lip >= crate::hud::LEDGE_H,
        "the hand's cast starts {} pixels above the shelf's lower edge",
        crate::hud::LEDGE_H - (down.0 + lip)
    );
    assert!(
        app.world().entity(shelf).get::<BoxShadow>().is_none(),
        "a `BoxShadow` on a translucent shelf lays its own rectangle \
         across the whole width of the window"
    );
}

/// A watching seat with `green` green mana floating and `prompt` on the
/// shelf.
///
/// The sentence is a parameter because the pool's tests all need to be
/// able to rebuild the shelf *without* touching the pool, which is the
/// premise the retained strip exists to survive.
fn pool_of(green: u32, prompt: &str) -> Duel {
    let mut duel = duel_watching();
    duel.last_error = Some(baylee_client_core::i18n::Refusal::Verbatim(
        prompt.to_string(),
    ));
    {
        let view = duel.view.as_mut().expect("the seat has a view");
        let seat = view.seat;
        view.seats
            .iter_mut()
            .find(|s| s.player == seat)
            .expect("this seat sits at its own table")
            .mana_pool
            .green = green;
    }
    crate::rebuild_board(&mut duel);
    duel
}

/// One of a player's button's edge lights, among `children`: how wide it
/// is drawn and at what alpha.
fn players_edge<M: Component>(app: &mut App, children: &[Entity]) -> (Val, f32) {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Node, &BackgroundColor), With<M>>();
    children
        .iter()
        .find_map(|c| q.get(app.world(), *c).ok())
        .map(|(node, ink)| (node.width, ink.0.alpha()))
        .expect("the edge is on the button")
}

/// The players' strip (#264): one button per seat, the reader's first,
/// each a [`PlayerTab`] so a press is the rim's press; and the three
/// edges on the seats they belong to — the turn's line on the active
/// seat, the breath on the awaited one, the camera's bar on the seat the
/// camera is on, which with no seat focused is the reader's own.
///
/// Then a life changes, and the buttons are the **same entities**: the
/// writing on them is rebuilt and the button is not, which is what keeps
/// a hover's warmth and the edges' movements running through it.
#[test]
fn the_players_strip_lists_every_seat_and_lights_the_three_edges() {
    let mut duel = duel_watching();
    {
        let view = duel.view.as_mut().expect("a view");
        view.active = PlayerId::new(1);
        view.awaiting = Some(PlayerId::new(1));
    }
    let mut app = bar_of(duel);
    app.update();

    let buttons = |app: &mut App| {
        let mut strips = app
            .world_mut()
            .query_filtered::<(&Children, &Visibility), With<ledge::players::PlayersStrip>>();
        let (kids, seen) = strips.single(app.world()).expect("one players' strip");
        assert_eq!(*seen, Visibility::Inherited, "the strip is up at a table");
        let kids: Vec<Entity> = kids.iter().collect();
        let mut q = app.world_mut().query::<(
            &ledge::players::PlayerButton,
            &PlayerTab,
            &BorderColor,
            &Children,
        )>();
        kids.into_iter()
            .filter_map(|kid| {
                q.get(app.world(), kid)
                    .ok()
                    .map(|(button, tab, border, children)| {
                        assert_eq!(
                            button.player, tab.player,
                            "a press frames the seat it names"
                        );
                        (
                            kid,
                            button.player,
                            *border,
                            children.iter().collect::<Vec<_>>(),
                        )
                    })
            })
            .collect::<Vec<_>>()
    };
    let was = buttons(&mut app);
    let seats: Vec<PlayerId> = was.iter().map(|(_, p, _, _)| *p).collect();
    assert_eq!(
        seats,
        vec![PlayerId::new(0), PlayerId::new(1)],
        "every seat, the reader first"
    );

    // Each edge on its seat, at its end: the harness asks for less
    // motion, so every movement stands where it arrives.
    let (mine, theirs) = (&was[0].3, &was[1].3);
    assert_eq!(
        players_edge::<ledge::players::TurnLine>(&mut app, theirs),
        (percent(100), 1.0),
        "their turn, their line"
    );
    assert!(
        players_edge::<ledge::players::TurnLine>(&mut app, mine).1 < f32::EPSILON,
        "and no line on mine"
    );
    assert!(
        players_edge::<ledge::players::CameraBar>(&mut app, mine).1 > 0.5,
        "the camera is on my seat"
    );
    assert!(
        players_edge::<ledge::players::CameraBar>(&mut app, theirs).1 < f32::EPSILON,
        "and not on theirs"
    );
    assert_ne!(
        was[1].2, was[0].2,
        "the awaited seat's border is not at rest"
    );
    the_wait_is_a_line_on_its_chip(&mut app);

    // A life changes: written again, on the same buttons.
    {
        let mut duel = app.world_mut().resource_mut::<Duel>();
        let view = duel.view.as_mut().expect("a view");
        view.seats[1].life -= 3;
    }
    app.update();
    let now = buttons(&mut app);
    assert_eq!(
        now.iter().map(|(e, _, _, _)| *e).collect::<Vec<_>>(),
        was.iter().map(|(e, _, _, _)| *e).collect::<Vec<_>>(),
        "a life total rebuilt the buttons, and the pointer's warmth went with them"
    );
}

/// The wait's teal line, set in under the turn's (the owner, 08.10.2026:
/// lines, no ☀ or ⌛): on the awaited seat's chip (seat 1) and nowhere
/// else, and no chip writes a glyph for either state.
fn the_wait_is_a_line_on_its_chip(app: &mut App) {
    let shown: Vec<(PlayerId, ledge::players::TagKind)> = app
        .world_mut()
        .query::<(&ledge::players::ChipTag, &Visibility, &BackgroundColor)>()
        .iter(app.world())
        .filter(|(tag, seen, ink)| {
            !tag.plate && **seen != Visibility::Hidden && ink.0.alpha() >= 0.5
        })
        .map(|(tag, ..)| (tag.player, tag.kind))
        .collect();
    assert!(
        shown.contains(&(PlayerId::new(1), ledge::players::TagKind::Priority)),
        "the awaited seat wears the wait's line: {shown:?}"
    );
    assert!(
        !shown
            .iter()
            .any(|(p, k)| *p == PlayerId::new(0) && *k == ledge::players::TagKind::Priority),
        "and mine does not: {shown:?}"
    );
    let glyphs = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .filter(|t| t.0.contains('\u{f185}') || t.0.contains('\u{f254}'))
        .count();
    assert_eq!(glyphs, 0, "no sun or hourglass on the strip");
}

/// The strip grows out of the shelf rather than appearing on it.
///
/// Three claims, and the first two are what a plain `from_bottom` would
/// break. On the frame the window opens the strip is drawn at
/// [`motion::ZOOM_FROM`] — the start of the arrival, not the end of it —
/// and it is pinned at the **bottom-right** corner (the right end since
/// #264), because a node fixed at the right margin that shrinks toward
/// its own middle slides left as it grows. Then the movement ends: the clock is advanced past
/// [`motion::ZOOM_IN`] and the strip is at full size, which is what makes
/// the first two an arrival rather than a strip permanently drawn 12%
/// short.
///
/// With `reduce_motion` it is at full size on that same first frame,
/// which is the counter-test for the first claim on its own terms.
#[test]
fn the_strip_grows_out_of_the_shelf_rather_than_appearing_on_it() {
    for (still, want) in [(false, motion::ZOOM_FROM), (true, 1.0)] {
        let mut app = bar_of(owing(false));
        app.world_mut()
            .resource_mut::<crate::prefs::Prefs>()
            .edit()
            .reduce_motion = still;
        assert!(!strip_is_shown(&mut app), "nothing owed yet");

        *app.world_mut().resource_mut::<Duel>() = owing(true);
        app.update();

        assert!(
            strip_is_shown(&mut app),
            "a payment window opened, so the strip is up"
        );
        let (scale, shift) = strip_pose(&mut app);
        assert!(
            (scale.x - want).abs() < 0.001 && (scale.y - want).abs() < 0.001,
            "reduce_motion {still}: the strip is drawn at {scale:?} and \
             {want} was wanted"
        );
        if !still {
            assert_eq!(
                shift,
                motion::from_bottom_right(want),
                "the strip grows out of the corner it is pinned at"
            );

            tick(&mut app, motion::ZOOM_IN + 0.01);
            let (scale, shift) = strip_pose(&mut app);
            assert!(
                (scale.x - 1.0).abs() < 0.001,
                "and the arrival ends at full size, not at {scale:?}"
            );
            assert_eq!(
                shift,
                motion::from_bottom_right(1.0),
                "with nothing left to correct for"
            );
        }
    }
}

/// And it folds back into the shelf rather than being taken off it.
///
/// The clock is advanced for this one: against the harness's own still
/// clock "the strip is still shown" would be true for the wrong reason. The
/// frame that reads the closed window starts the fold at full size, the
/// strip is **still there** through it, and only past [`motion::ZOOM_OUT`]
/// is it away.
#[test]
fn the_strip_folds_back_into_the_shelf_rather_than_being_taken_off_it() {
    let mut app = bar_of(owing(true));
    app.world_mut()
        .resource_mut::<crate::prefs::Prefs>()
        .edit()
        .reduce_motion = false;
    tick(&mut app, motion::ZOOM_IN + 0.01);
    assert!(strip_is_shown(&mut app), "a payment window is open");

    *app.world_mut().resource_mut::<Duel>() = owing(false);
    tick(&mut app, 0.0);
    assert!(
        strip_is_shown(&mut app),
        "the strip folds away rather than being taken away"
    );
    let (scale, _) = strip_pose(&mut app);
    assert!(
        (scale.x - 1.0).abs() < 0.001,
        "and the fold begins at full size, not at {scale:?}"
    );

    tick(&mut app, motion::ZOOM_OUT + 0.01);
    assert!(
        !strip_is_shown(&mut app),
        "and once the fold is over the strip is away"
    );
}

/// One frame, with `seconds` of clock in front of it.
pub(super) fn tick(app: &mut App, seconds: f32) {
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs_f32(seconds));
    app.update();
}

/// The strip's scale and where that scale is anchored.
fn strip_pose(app: &mut App) -> (Vec2, Val2) {
    let mut q = app
        .world_mut()
        .query_filtered::<&UiTransform, With<ledge::pool::PoolStrip>>();
    let at = q.single(app.world()).expect("one strip");
    (at.scale, at.translation)
}
