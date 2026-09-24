use super::*;
use baylee_client_core::prose::bracketed;
use baylee_client_core::test_support::{ViewBuilder, printed};
use baylee_core::ids::PlayerId;

fn statics() -> GameStatic {
    GameStatic {
        decision_secs: None,
        reconnect_secs: None,
        view_version: baylee_view::VIEW_VERSION,
        game_id: "g".into(),
        your_seat: PlayerId::new(0),
        seats: vec![baylee_view::SeatIdentity {
            player: PlayerId::new(1),
            display_name: "House AI".into(),
            is_ai: true,
            away: false,
            team: None,
        }],
        prints: vec![],
    }
}

/// The sheet a player has never moved is at least one whole row wide.
///
/// `Placement::DEFAULT_W` is a number in the renderer-free half, where it
/// can be tested but where a row's columns do not exist; the arithmetic
/// that produced it lives here. This is the seam between them, so a column
/// widened on one side cannot silently leave the other with a name that
/// no longer has its measure.
///
/// A **floor** and not an equality, and this doc said "exactly" until the
/// two numbers were read side by side: a row is 702 and the sheet opens at
/// 1020, because the slack past a row goes to the columns that flex and more
/// of it is more name. What the seam has to stop is the sheet opening
/// *narrower* than the row it holds, which is a name column with nothing left
/// in it — and that is exactly what a floor says.
#[test]
fn the_default_width_is_one_whole_row() {
    let off = Placement::DEFAULT_W - TRAY_PANEL_W;
    assert!(
        off >= 0.0,
        "a row is {TRAY_PANEL_W}, the sheet opens at {} ({off} out)",
        Placement::DEFAULT_W
    );
}

/// And it opens showing half of a ninth row.
///
/// The same seam one axis over, and the half is the whole point: a list
/// cut off at a row boundary looks like a list that ends there.
#[test]
fn the_default_height_shows_half_a_row() {
    let want = TRAY_CHROME_H + TRAY_ROWS * TRAY_ROW_H;
    let off = (Placement::DEFAULT_H - want).abs();
    assert!(
        off <= 1.0,
        "{TRAY_ROWS} rows is {want}, the sheet opens at {} ({off} out)",
        Placement::DEFAULT_H
    );
    // Genuinely half: the list is cut through a row rather than between
    // two, which is what says there is more below.
    let spare = (Placement::DEFAULT_H - TRAY_CHROME_H) % TRAY_ROW_H;
    assert!(
        spare > TRAY_ROW_H * 0.25 && spare < TRAY_ROW_H * 0.75,
        "the bottom row is cut at {spare} of {TRAY_ROW_H}, which reads as a whole one"
    );
}

/// The smallest sheet still has a row in it worth reading.
///
/// Both floors are one row's arithmetic: the width is the fixed columns
/// plus ten characters of name, and the height is the chrome plus two
/// whole rows. A sheet dragged smaller than either is a sheet with no
/// list left in it.
#[test]
fn the_floor_is_a_row_that_can_still_be_read() {
    let fixed = TRAY_PANEL_W - TRAY_NAME_W - TRAY_TYPE_W;
    let ten = 10.0 * TRAY_CH * TRAY_NAME_SIZE;
    assert!(
        (Placement::MIN_W - (fixed + ten)).abs() <= 1.0,
        "ten characters of name is {}, the floor is {}",
        fixed + ten,
        Placement::MIN_W
    );
    let two = TRAY_CHROME_H + 2.0 * TRAY_ROW_H;
    assert!(
        (Placement::MIN_H - two).abs() <= 1.0,
        "two rows is {two}, the floor is {}",
        Placement::MIN_H
    );
}

/// The band never claims more room than the window has.
#[test]
fn the_band_is_what_is_left_above_the_hand() {
    // A window the size the dev harness reports.
    let tall = 1052.0 - EDGE - HAND_ZONE_H;
    assert!(tall > Placement::MIN_H, "the fixture is not exercising it");
    // A window too short for a sheet still gets one: `MIN_H` wins, and a
    // sheet clamped to nothing would be a sheet that is not there.
    let cramped = (200.0f32 - EDGE - HAND_ZONE_H).max(Placement::MIN_H);
    assert!((cramped - Placement::MIN_H).abs() < f32::EPSILON);
}

/// The two bands that reach the panel's corners are cut with the panel's
/// own curve, one pixel in.
///
/// `Overflow::clip()` clips rectangularly in `bevy_ui`, so this is not
/// something the panel can do for its children: a square child paints
/// over the rounded corner it sits in *and* over the arc of the border,
/// which is what the owner saw on 14.09.2026 — a square corner with two
/// straight lines stopping short of each other. The number is derived
/// from [`super::SHEET_R`] rather than typed, so that the two curves
/// cannot drift apart; this is the assertion that the derivation is the
/// concentric one and not merely equal.
#[test]
fn the_head_is_cut_concentrically_with_the_panel() {
    // One pixel of border between the two curves, which is what
    // `spawn_tray`'s panel carries and what `spawn_footer` sits inside.
    assert!(
        (TRAY_HEAD_R - (TRAY_RADIUS - 1.0)).abs() < f32::EPSILON,
        "the head's radius is {TRAY_HEAD_R} inside a {TRAY_RADIUS} panel with \
         a 1 px border, so the two curves are not concentric"
    );
    let source = include_str!("../tray.rs");
    for band in [
        "border_radius: BorderRadius::top(px(TRAY_HEAD_R))",
        "border_radius: BorderRadius::bottom(px(TRAY_HEAD_R))",
    ] {
        assert!(
            source.contains(band),
            "the band spelled `{band}` no longer carries the panel's curve"
        );
    }
}

/// No view asks for a picture bigger than the one the client has.
///
/// The whole cost of the two larger views is here, and it is a cost that
/// has to stay at zero: every row already fetches its art at
/// [`ArtSize::Small`], so drawing it larger costs no texture at all —
/// until a size crosses what that image holds, at which point the honest
/// fix is the next size up and a hundred-card library at
/// [`ArtSize::Normal`] is 133 MB against a 96 MB budget on a phone.
///
/// The bound is stated in *physical* pixels and against the window's own
/// scale, because that is the comparison that means anything: a 73-wide
/// picture at scale 2 is 146 across, which is exactly what Scryfall
/// serves. The cap is allowed to reach past 1:1, and by how much is the
/// number this test pins.
///
/// [`ArtSize::Small`]: baylee_client_core::images::ArtSize::Small
/// [`ArtSize::Normal`]: baylee_client_core::images::ArtSize::Normal
#[test]
fn the_larger_views_stay_inside_the_one_size_the_art_has() {
    use baylee_client_core::images::ArtSize;
    let (art_w, _) = ArtSize::Small.dimensions();
    #[allow(clippy::cast_precision_loss)]
    let art_w = art_w as f32;
    // This machine, and every retina screen: two device pixels to one
    // logical one.
    let scale = 2.0;

    assert!(
        (TRAY_BIG_THUMB_W * scale - art_w).abs() < 0.5,
        "the large list draws {TRAY_BIG_THUMB_W} logical px, which is {} device px against \
         the {art_w} the image has — it is meant to be exactly 1:1",
        TRAY_BIG_THUMB_W * scale
    );
    assert!(
        TRAY_TILE_MAX * scale / art_w < 1.4,
        "a grid tile may grow to {TRAY_TILE_MAX} px, which upscales the art {:.2}× — past \
         about 1.4 the softening is what a player sees, and the answer is not a bigger \
         ArtSize",
        TRAY_TILE_MAX * scale / art_w
    );
    const {
        assert!(
            TRAY_TILE_MAX >= TRAY_BIG_THUMB_W,
            "a tile cannot be capped below the size it starts at"
        );
        // And the detailed row, which is the one that was measured first
        // and must stay under both.
        assert!(TRAY_THUMB_W < TRAY_BIG_THUMB_W);
    }
}

/// The grid fills the sheet at every width the sheet can be dragged to.
///
/// Three measures, all of them the sheet's own: its floor, the width it
/// opens at, and a maximised sheet on this screen. A column count of one
/// at the floor would be a grid that is a list with the words taken out.
///
/// What it packs is the **tile**, chrome and all, exactly as
/// [`spawn_grid`] does — which is the half that was wrong once. Sharing
/// the measure out among pictures and then drawing each of them eight
/// pixels wider puts every full row over its measure by a tile, and
/// `bevy_ui` answers that by wrapping the last one onto a line of its
/// own. Nothing about a zone holding two cards can show it.
#[test]
fn the_grid_fills_the_sheet_at_its_floor_and_at_its_default() {
    use baylee_client_core::browser::Placement;
    let at = |sheet: f32| {
        let (across, tile, air) = grid_across(
            sheet - 2.0 * TRAY_SIDE,
            TRAY_BIG_THUMB_W + TRAY_TILE_CHROME,
            TRAY_TILE_MAX + TRAY_TILE_CHROME,
            TRAY_TILE_GAP,
        );
        #[allow(clippy::cast_precision_loss)]
        let used = across as f32 * tile + (across - 1) as f32 * air;
        (across, tile - TRAY_TILE_CHROME, used)
    };

    let (floor, _, used) = at(Placement::MIN_W);
    assert!(
        floor >= 3,
        "the sheet at its floor shows {floor} tiles across, which is not a grid"
    );
    assert!(used <= Placement::MIN_W - 2.0 * TRAY_SIDE + 0.01);

    let (default, _, used) = at(Placement::DEFAULT_W);
    assert!(
        default > floor,
        "the sheet at {} shows no more tiles than at its floor",
        Placement::DEFAULT_W
    );
    assert!(used <= Placement::DEFAULT_W - 2.0 * TRAY_SIDE + 0.01);

    // A maximised sheet on this machine's window.
    let (wide, _, used) = at(1728.0);
    assert!(wide > default);
    assert!(used <= 1728.0 - 2.0 * TRAY_SIDE + 0.01);

    // The row always *fills* its measure, at every width in between —
    // there is no ragged right edge and no width at which the grid pays
    // for its own gaps twice. Swept rather than sampled, because the two
    // places this can go wrong are the fencepost (n tiles, n-1 gaps) and
    // the step where a column is gained, and both are one pixel wide.
    let mut capped = 0u32;
    for w in (Placement::MIN_W as u16)..=2000 {
        let sheet = f32::from(w);
        let measure = sheet - 2.0 * TRAY_SIDE;
        let (across, art, used) = at(sheet);
        assert!(
            (TRAY_BIG_THUMB_W..=TRAY_TILE_MAX).contains(&art),
            "a sheet of {sheet} draws a {art}-wide picture"
        );
        assert!(used <= measure + 0.01, "{across} tiles overflow {measure}");
        assert!(
            used >= measure - 0.01,
            "a sheet of {sheet} leaves {} px of its row empty, which is a grid paying for \
             a gap it does not have",
            measure - used
        );
        capped += u32::from(art >= TRAY_TILE_MAX);
    }
    // The cap does bite, and it bites in one narrow window: a row of
    // three shares out at most one 81-plus-gap between them, so three
    // columns can reach 100 and four never can — above about a 386-pixel
    // sheet the picture's own floor is what decides every width. That is
    // worth a count rather than a bound, because both halves are
    // load-bearing. A zero here means the cap is unreachable and the
    // slack-into-the-gaps rule is dead code on this panel; a number in
    // the hundreds means the pictures are pinned at 100 and the grid is
    // paying for its width in air instead of in cards.
    assert!(
        (1..=32).contains(&capped),
        "the cap binds at {capped} of the sheet's widths, which is not the narrow window \
         these three numbers describe"
    );
}

/// A tab says how many cards are in it, and says it in the one register
/// the dialog greys.
///
/// Two claims in one, because they are one decision: the count is drawn
/// as an aside rather than as part of the name, and [`dialog_text`]
/// decides that by finding a bracket. A count appended with a separator
/// would read at full ink weight and make every tab look twice as long.
#[test]
fn a_zone_tab_carries_its_count_as_an_aside() {
    let view = ViewBuilder::new(2)
        .with_graveyard(0, vec![printed(1, 0, "Llanowar Elves", 1)])
        .with_graveyard(1, vec![printed(2, 1, "Ponder", 2)])
        .build();
    let mine = zone_label(
        Lang::En,
        BrowseZone::Graveyard(PlayerId::new(0)),
        &view,
        &statics(),
    );
    assert_eq!(mine, "Graveyard (1)", "my own pile does not say whose");

    let runs: Vec<_> = bracketed(&mine).collect();
    assert_eq!(
        runs,
        vec![("Graveyard ", false), ("(1)", true)],
        "the count is not in the aside register the dialog greys"
    );

    let theirs = zone_label(
        Lang::En,
        BrowseZone::Graveyard(PlayerId::new(1)),
        &view,
        &statics(),
    );
    assert_eq!(
        theirs, "Graveyard · House AI (1)",
        "somebody else's pile says whose, and still counts"
    );
}

/// The footer is exactly what the question allows, and nothing more.
///
/// Three claims, one per state, and they are the whole of §6's footer.
/// **Confirm is a control only when the answer is complete** — an unlit
/// one carries no `Button` at all, for the reason a pinned zone tab
/// carries none: a thing that lights under the pointer and then refuses
/// the click is worse than one that never invited it. And **Cancel is
/// drawn only when the minimum is zero**, because there is no cancel on
/// the wire: it is `Interaction::confirm` sending an empty answer, so a
/// question that will not take one has no way out to offer.
#[test]
fn the_footer_offers_only_what_the_question_allows() {
    use baylee_core::ids::{ObjectId, PlayerId};
    use baylee_engine::choice::{ChoicePrompt, Pending};

    fn asked(min: u8, picks: &[u32]) -> baylee_client_core::Interaction {
        let mut it = baylee_client_core::Interaction::new(
            Pending::ChooseCards {
                player: PlayerId::new(0),
                options: (1..4).map(|n| ObjectId::new(n, 0)).collect(),
                min,
                max: 3,
                prompt: ChoicePrompt::SearchLibrary,
            },
            PlayerId::new(0),
        );
        for id in picks {
            it.toggle(ObjectId::new(*id, 0));
        }
        it
    }

    /// Builds one footer and reports `(confirm is a control, cancel is drawn)`.
    fn footer_of(it: &baylee_client_core::Interaction) -> (bool, bool) {
        let mut app = App::new();
        let fonts = UiFonts {
            text: Handle::default(),
            medium: Handle::default(),
            bold: Handle::default(),
            italic: Handle::default(),
            medium_italic: Handle::default(),
            serif: Handle::default(),
            serif_italic: Handle::default(),
            icons: Handle::default(),
            mana: Handle::default(),
        };
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let foot = {
            let mut commands = Commands::new(&mut queue, app.world());
            spawn_footer(&mut commands, &fonts, Lang::En, Some(it)).expect("a question has one")
        };
        queue.apply(app.world_mut());
        let kids: Vec<_> = app
            .world()
            .entity(foot)
            .get::<Children>()
            .expect("a footer has buttons")
            .iter()
            .collect();
        let lit = kids
            .iter()
            .any(|e| app.world().entity(*e).contains::<PromptButton>());
        let out = kids
            .iter()
            .any(|e| app.world().entity(*e).contains::<TrayNone>());
        (lit, out)
    }

    assert_eq!(
        footer_of(&asked(1, &[])),
        (false, false),
        "an incomplete answer offered a Confirm, or a way out that does \
         not exist"
    );
    assert_eq!(
        footer_of(&asked(1, &[1])),
        (true, false),
        "a complete answer could not be sent"
    );
    assert_eq!(
        footer_of(&asked(0, &[])),
        (true, true),
        "a question that takes an empty answer drew no way out"
    );
}

/// W2's whole claim, written as an order: the veil goes over what answers
/// nothing and under everything that does.
///
/// One assertion rather than five literals in four files, because that is
/// the failure it guards. A `ZIndex` is local to a parent's children, so
/// these five mean something only *against each other* — a veil written
/// as a 3 beside a prompt slip that had never been given a number at all
/// is a dim drawn over the sentence stating the question.
///
/// In `const` blocks, so the order is checked when the crate is *built*
/// and not when its tests are run. It is still a named test because the
/// rule wants somewhere to be written down in words, and because a
/// constant that silently stopped being compared would be no rule at all.
#[test]
fn the_veil_lies_over_the_table_and_under_the_question() {
    const {
        assert!(
            Z_STACK < Z_VEIL && Z_HAND < Z_VEIL,
            "the stack and the hand answer nothing here and go dark with the table"
        );
        assert!(
            Z_VEIL < Z_LEDGE,
            "the ledge carries the question, and a dimmed question is one a player is told not to answer"
        );
        assert!(
            Z_LEDGE < Z_SHEET,
            "the dialog is what the question is about"
        );
        assert!(
            Z_SHEET < Z_PREVIEW,
            "a card held up to the light is held over whatever raised it"
        );
    }
}

/// The fade rises to exactly the veil's own alpha and falls back to
/// nothing — and it survives the rebuild that every tick of a checkbox
/// causes, which is the whole reason the number is not on the node.
#[test]
fn the_veil_rises_while_the_question_stands_and_falls_when_it_is_answered() {
    use baylee_client_core::test_support::{ViewBuilder, printed};
    use baylee_core::ids::{ObjectId, PlayerId};
    use baylee_engine::choice::{ChoicePrompt, Pending};
    use std::time::Duration;

    fn tick(app: &mut App) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(16));
        app.update();
    }
    fn alpha(app: &mut App) -> f32 {
        let mut found = app
            .world_mut()
            .query_filtered::<&BackgroundColor, With<TableVeil>>();
        found.iter(app.world()).next().expect("a veil").0.alpha()
    }
    fn raise(app: &mut App) {
        let mut queue = bevy::ecs::world::CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue, app.world());
            spawn_veil(&mut commands);
        }
        queue.apply(app.world_mut());
    }

    let mut app = App::new();
    app.init_resource::<Time>()
        .init_resource::<Veil>()
        .init_resource::<crate::Duel>()
        .add_systems(Update, dim_the_table);
    raise(&mut app);
    assert!(alpha(&mut app).abs() < 1e-6, "it is spawned clear");

    // A search, through the same door the client uses: cards shown, every
    // answer among them.
    let view = ViewBuilder::new(2)
        .with_looking_at((10..13).map(|s| printed(s, 0, "Forest", 1)).collect())
        .build();
    let search = baylee_client_core::Interaction::new(
        Pending::ChooseCards {
            player: PlayerId::new(0),
            options: (10..13).map(|n| ObjectId::new(n, 0)).collect(),
            min: 1,
            max: 1,
            prompt: ChoicePrompt::SearchLibrary,
        },
        PlayerId::new(0),
    );
    app.world_mut()
        .resource_mut::<crate::Duel>()
        .browser
        .follow(&view, Some(&search));
    for _ in 0..3 {
        tick(&mut app);
    }
    let rising = alpha(&mut app);
    assert!(
        rising > 0.0 && rising < palette::TABLE_VEIL.alpha(),
        "three frames in it should be on its way and not there yet: {rising}"
    );

    // A tick of a checkbox rebuilds the whole overlay, veil included.
    let standing: Vec<_> = {
        let mut found = app.world_mut().query_filtered::<Entity, With<TableVeil>>();
        found.iter(app.world()).collect()
    };
    for entity in standing {
        app.world_mut().entity_mut(entity).despawn();
    }
    raise(&mut app);
    tick(&mut app);
    assert!(
        alpha(&mut app) > rising,
        "the rebuilt veil started again from nothing — the fade is on the \
         node instead of in `Veil`"
    );

    for _ in 0..60 {
        tick(&mut app);
    }
    assert!(
        (alpha(&mut app) - palette::TABLE_VEIL.alpha()).abs() < 1e-4,
        "it settles at the veil's own alpha, not a hair under it"
    );

    // Answered. The number falls whether or not a node is left to paint.
    app.world_mut().resource_mut::<crate::Duel>().browser = Browser::new();
    for _ in 0..60 {
        tick(&mut app);
    }
    assert!(
        alpha(&mut app).abs() < 1e-6,
        "and is clear again for the next one"
    );
}

/// Nothing on the dialog is drawn in the teal the redesign retires.
///
/// `palette::ACCENT` is what "this is asking you something" used to be
/// said in, and §1 gives that job to candle at two energies. The check is
/// on the source because what is being held is a rule about the whole
/// file, not about one node — and it is the counterpart of
/// `the_sheet_writes_no_letters_in_brass`, which holds the same kind of
/// rule over the two parchment surfaces.
#[test]
fn the_dialog_says_nothing_in_teal() {
    // Assembled rather than written out, or the needle is in the
    // haystack and this test fails on its own source line.
    let teal = format!("palette::{}", "ACCENT");
    for line in include_str!("../tray.rs").lines() {
        let code = line.split("//").next().unwrap_or(line);
        assert!(
            !code.contains(&teal),
            "the accent is the teal §1 retires: {line}"
        );
    }
}

/// A row is lit by the predicate the table lights the same card by, in the
/// hand's two colours (#242).
///
/// Two halves because they are two joins: [`pile_reach`] is what the sheet's
/// rebuild gate compares and what its rows read, and [`light_thumb`] is the
/// colour each answer becomes. A sheet that drew every row indigo passes the
/// first and fails the second; one that asked a different question than the
/// felt fails the first.
#[test]
fn a_row_is_lit_by_the_offer_the_table_draws() {
    use crate::flashback_reach_tests::{BURIED, buried, table};

    assert_eq!(
        pile_reach(&table(1, vec![buried(0, "Opt", Some("{U}"))])),
        vec![(BURIED, crate::Reach::Taps)],
        "Opt with Snapcaster's flashback on it, and one land to pay"
    );
    assert!(
        pile_reach(&table(1, vec![buried(0, "Opt", None)])).is_empty(),
        "and without it, a card nobody offers"
    );

    let mut world = World::new();
    let offered = world.spawn_empty().id();
    let taps = world.spawn_empty().id();
    let dark = world.spawn_empty().id();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    {
        let mut commands = Commands::new(&mut queue, &world);
        light_thumb(&mut commands, offered, Some(crate::Reach::Offered));
        light_thumb(&mut commands, taps, Some(crate::Reach::Taps));
        light_thumb(&mut commands, dark, None);
    }
    queue.apply(&mut world);
    let tint = |entity: Entity| {
        world
            .get::<BoxShadow>(entity)
            .map(|shadow| shadow.0[0].color.with_alpha(1.0))
    };
    assert_eq!(tint(offered), Some(palette::ACTIVE), "the engine's gold");
    assert_eq!(tint(taps), Some(palette::REACHABLE), "this client's indigo");
    assert_eq!(tint(dark), None);
}
