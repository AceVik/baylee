//! The mana pool and players strips on the shelf.

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

/// The mana pool is a badge again, and this time on purpose.
///
/// The turn is the point of the test, so it is written down rather than
/// swapped out. The chip this began as was drawn "while the seat has
/// something to answer" and blinked out at every opponent's priority,
/// which AX §4.1 called movement carrying no information; the column that
/// replaced it stood always, on the argument that the shelf's left edge
/// was reserved whatever was on it. Off the shelf that argument has
/// nothing left to rest on, and the owner asked for the other rule back
/// on its own terms — *"Es ist hidden, wenn kein Mana im Mana Pool ist
/// und ist nur dann sichtbar, wenn dort Mana drin ist"*. What is not the
/// chip's rule is the **condition**: it comes and goes with the *mana*
/// and not with whose priority it is, so a watching seat holding mana
/// still sees it, which is the case this test opens with.
///
/// Then the two claims that did not turn: mana in it is a numeral beside
/// a disc rather than a row of discs to count, and a restricted mana is
/// an entry of its own (CR 106.6).
#[test]
fn the_mana_pool_is_drawn_only_while_something_is_floating() {
    let label = Phrase::ManaPool.text(Lang::En).to_string();

    let mut watching = bar_of(duel_watching());
    assert!(
        !strip_is_shown(&mut watching),
        "nothing is floating, so there is no strip to read"
    );
    assert!(
        !said(&mut watching).contains(&"\u{2014}".to_string()),
        "the em dash the strip replaces is gone, not merely hidden"
    );

    // And with mana in it the strip is up and the count is a numeral.
    let mut duel = duel_watching();
    {
        let view = duel.view.as_mut().expect("the seat has a view");
        let seat = view.seat;
        let pool = &mut view
            .seats
            .iter_mut()
            .find(|s| s.player == seat)
            .expect("this seat sits at its own table")
            .mana_pool;
        pool.green = 3;
        pool.restricted[baylee_core::mana::ManaColor::White.index()] = 1;
    }
    crate::rebuild_board(&mut duel);
    let mut floating = bar_of(duel);
    assert!(
        strip_is_shown(&mut floating),
        "this seat is still only watching, and it is holding mana: the \
         strip follows the pool and not the priority"
    );
    let lines = said(&mut floating);
    assert!(
        lines.contains(&label),
        "the strip says what it is once it is up: {lines:?}"
    );
    assert!(
        lines.contains(&"\u{00d7}3".to_string()) && lines.contains(&"\u{00d7}1".to_string()),
        "three green and one restricted white, as numerals: {lines:?}"
    );
}

/// Whether the mana pool's strip is drawn at all.
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

/// A mana that is floating is one entity for as long as it is floating.
///
/// The whole reason `ledge/pool.rs` exists, and the one claim that cannot
/// be made about anything else on this shelf. §4.1 wants a new entry to
/// pop and a spent one to fade, and neither is possible while the thing
/// being animated is despawned and rebuilt whenever the *sentence*
/// changes — which is at every priority. So: the same entity across a
/// rebuild that changed both the question and the count, and no entity at
/// all once the mana is spent.
///
/// The counter-test is in the middle of it. The shelf really is rebuilt
/// between the two readings — the sentence is a different sentence — so
/// an entry that survives is surviving something, rather than sitting in
/// a tree nothing touched.
#[test]
fn a_floating_mana_is_one_entity_for_as_long_as_it_floats() {
    let mut app = bar_of(pool_of(1, "one"));
    let first = pool_entries(&mut app);
    assert_eq!(first.len(), 1, "one colour is floating, so one entry");
    let before = said(&mut app).join("|");

    *app.world_mut().resource_mut::<Duel>() = pool_of(2, "two");
    app.update();
    assert_eq!(
        pool_entries(&mut app),
        first,
        "the same mana, more of it: the entry is written, not replaced"
    );
    assert_ne!(
        said(&mut app).join("|"),
        before,
        "this test's premise is that the shelf was rebuilt under it"
    );

    // Spent. With motion off the fade is over on the frame it starts.
    *app.world_mut().resource_mut::<Duel>() = pool_of(0, "three");
    app.update();
    assert!(
        pool_entries(&mut app).is_empty(),
        "a spent mana leaves, rather than being left behind"
    );
    // And the strip waits for it: one more frame, because it can only go
    // away once nothing is standing on it.
    app.update();
    assert!(
        !strip_is_shown(&mut app),
        "the strip goes with the last pip, rather than standing empty"
    );
}

/// And it survives the *other* rebuild, which is the one the strip moved
/// under.
///
/// The test above changes the sentence, which rebuilds the shelf; this
/// one moves the pointer, which is what [`HudRevision`] counts and what
/// tears down every child of the root. While the pool was a column on the
/// shelf those were the same claim made twice, and the exemption that
/// mattered was `sync_ledge`'s. The strip hangs off the root now, so the
/// exemption that matters is `sync_overlay`'s own sweep — a different
/// list, in a different file, that nothing else here would notice the
/// absence of: a pool rebuilt on every pointer move still *draws*, and
/// only the pop and the fade are lost, which no test that reads the row
/// can see.
///
/// The second assertion is the counter-test, and it is the same one the
/// sweep's own test makes: the root really did rebuild between the two
/// readings, so an entry that came through came through something.
#[test]
fn a_floating_mana_survives_the_pointer_crossing_a_card() {
    let mut app = bar_of(pool_of(1, "one"));
    let first = pool_entries(&mut app);
    assert_eq!(first.len(), 1, "one colour is floating, so one entry");

    // The counter-test, hung under the root by hand. A watching seat with
    // one mana and nothing else draws *nothing* of its own up there — the
    // hand, the stack and the preview are all absent — so "the nodes that
    // were rebuilt" is an empty list on both sides of the update and
    // proves nothing at all. A node the sweep has never been told about
    // is the witness instead: it is gone afterwards exactly when the
    // sweep ran, which is the premise, and the strip beside it came
    // through the same sweep, which is the claim.
    let root = {
        let mut q = app.world_mut().query_filtered::<Entity, With<HudRoot>>();
        q.single(app.world()).expect("one overlay root")
    };
    let decoy = app.world_mut().spawn(Node::default()).id();
    app.world_mut().entity_mut(root).add_child(decoy);

    app.world_mut().resource_mut::<Duel>().hovered = Some(ObjectId::new(1, 0));
    app.update();

    assert!(
        app.world().get_entity(decoy).is_err(),
        "this test's premise is that the root was swept under it"
    );
    assert_eq!(
        pool_entries(&mut app),
        first,
        "the pointer moved and the mana was rebuilt with the overlay, so \
         the pop and the fade have nothing left to animate"
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

fn pool_entries(app: &mut App) -> Vec<Entity> {
    let mut q = app.world_mut().query::<(Entity, &ledge::pool::PoolEntry)>();
    q.iter(app.world()).map(|(e, _)| e).collect::<Vec<_>>()
}

/// The strip grows out of the shelf rather than appearing on it.
///
/// Three claims, and the first two are what a plain `from_bottom` would
/// break. On the frame the first mana arrives the strip is drawn at
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
        let mut app = bar_of(pool_of(0, "none"));
        app.world_mut()
            .resource_mut::<crate::prefs::Prefs>()
            .edit()
            .reduce_motion = still;
        assert!(!strip_is_shown(&mut app), "nothing floating yet");

        *app.world_mut().resource_mut::<Duel>() = pool_of(1, "one");
        app.update();

        assert!(
            strip_is_shown(&mut app),
            "a mana arrived, so the strip is up"
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
/// The clock has to be advanced for this one, and that is the finding
/// rather than a detail. Written against the harness's own still clock it
/// read as a test and asserted nothing: the strip's fold cannot start
/// until the row is empty, the last pip cannot finish fading while
/// `delta` is zero, so with the movement on the fold was never reached
/// and "the strip is still shown" was true for the wrong reason. It
/// passed against a `grow_the_pool` that hid the strip the instant the
/// pool emptied — the exact fault it was written for.
///
/// So the two movements are walked through in order, each with the clock
/// pushed past its own span: the pip leaves, the frame after that reads
/// an empty row and starts the fold, and the strip is **still there**
/// through it. Only past [`motion::ZOOM_OUT`] again is it away.
#[test]
fn the_strip_folds_back_into_the_shelf_rather_than_being_taken_off_it() {
    let mut app = bar_of(pool_of(1, "one"));
    app.world_mut()
        .resource_mut::<crate::prefs::Prefs>()
        .edit()
        .reduce_motion = false;
    app.update();
    assert!(strip_is_shown(&mut app), "one mana is floating");

    // Spent. The pip's own fade runs first, because the strip must not
    // fold around something still standing on it.
    *app.world_mut().resource_mut::<Duel>() = pool_of(0, "none");
    tick(&mut app, motion::ZOOM_OUT + 0.01);
    assert!(pool_entries(&mut app).is_empty(), "the pip has gone");
    assert!(
        strip_is_shown(&mut app),
        "and the strip is still up: nothing has read the empty row yet"
    );

    // The frame that reads it and starts the fold, with no time in it —
    // so the strip is at the beginning of its own movement and not past
    // the end of it.
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

/// The strip never goes away around a pip that is still on it.
///
/// The same claim this made about the em dash, on the thing that replaced
/// it — and it got *sharper* in the substitution, which is why the test
/// stayed. A dash drawn over a fading pip was a second reading of the row
/// beside the first; a strip taken away over one deletes the fade
/// outright, so the pip the player spent vanishes instead of leaving.
///
/// The bug it guards is one missing clause. On the spend frame the pool
/// names nothing and nothing is *yet* marked closing, so a reading that
/// asks only "is anything fading" answers "the row is empty" on the one
/// frame where the row is at its fullest.
///
/// It needs the movement **on**, which is why it is a second test rather
/// than two more lines in the one above. With `reduce_motion` the spent
/// pip is despawned before the frame ends, so a strip gone beside it is
/// right by accident; here the harness clock never advances, so the
/// fading entry sits at the start of its fade for as long as the test
/// looks at it. The first assertion is the counter-test: the pip really
/// is still there, so the strip that stayed up is the strip *waiting*
/// rather than a row that was never emptied.
#[test]
fn the_strip_does_not_go_away_over_a_pip_that_is_still_fading() {
    let mut app = bar_of(pool_of(1, "one"));
    app.world_mut()
        .resource_mut::<crate::prefs::Prefs>()
        .edit()
        .reduce_motion = false;

    *app.world_mut().resource_mut::<Duel>() = pool_of(0, "two");
    app.update();
    assert_eq!(
        pool_entries(&mut app).len(),
        1,
        "the spent mana is still on the row, fading"
    );
    assert!(
        strip_is_shown(&mut app),
        "so the strip must not be taken out from under it"
    );
}
