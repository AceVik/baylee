//! The end of a game, on a sheet over a darkened table.
//!
//! Before this the client said a game was over the way it says everything
//! else: one line in the prompt bar at thirteen pixels, while the board went
//! on being drawn underneath and two buttons floated over it sixty-four
//! pixels from the top of the window. A result is not a prompt — there is
//! nothing to answer — so it does not belong in the bar that holds questions,
//! and the bar goes quiet while this stands (`overlay::sync_overlay`).
//!
//! # Parchment, and one hue for all three endings
//!
//! A result is *read*, so it is a sheet and not a dialog, and the prompt slip
//! already settled that a read sheet may carry its own answers — this is that
//! slip grown up, in the same paper with the same grain, shadow and radius.
//!
//! Won, lost and drawn are **the same sheet**; only the sentence differs.
//! Every hue in this palette already carries a claim — `INK_DANGER` is
//! damage, `HEAL` is life gained, `BRASS` is a thing already taken, `CANDLE`
//! is an offer — so a red "you lost" would read as damage and a gold "you
//! won" as something taken. `docs/redesign-proposal.md` §1 retires hues; it
//! does not hand out new ones. Size carries the feeling instead: forty
//! pixels, against the twenty this overlay never used to go above.
//!
//! # Why it is spawned once and not synchronised
//!
//! Everything else here is a retained tree with a revision, because
//! everything else changes: a hover, a card drawn, a life total. A result does
//! not. `Pending::GameOver` is the last thing the engine ever says at this
//! table, so the screen is built on `OnEnter(DuelPhase::Finished)`, taken down
//! on the way out, and never touched in between.
//!
//! # Two plugins, one composition
//!
//! The verdict is the *duel's* to say and the way out is the *shell's*:
//! `DuelPlugin` is meant to be embeddable in an application that already has a
//! front door, and it cannot know whether there is a lobby behind it. So this
//! draws the sheet and leaves one row in it marked [`FinishExits`], and
//! `lobby::ui::spawn_leave_button` puts its buttons there when a lobby is what
//! is behind this duel. The row has a **fixed height** so that the sheet does
//! not reflow when those buttons land a frame later. A launch handed a
//! `SeatTicket` adds no `LobbyPlugin` at all (`main.rs`), and then the row
//! stays empty — which is honest, because that client has nowhere to go
//! either.
//!
//! The **keyboard** way out is the shell's for the same reason and lives
//! beside those buttons, in `lobby::systems::leave_keys`: `DuelSet::Input`
//! runs only in `DuelPhase::Playing`, so every binding is off while this
//! sheet is up, and for a while that made it the one screen a keyboard could
//! reach and not leave. It reads the row rather than a list of its own, so
//! the two cases above stay one rule.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;
use baylee_client_core::interaction::{ending_reason, verdict};
use bevy::text::LineHeight;

/// How wide the sheet is drawn, at a window with room for it.
///
/// Wider than the ability sheet's 300, and it should be: that one stands
/// beside a card and has to leave the board readable, and this is the only
/// thing on the screen.
///
/// The number is the longest verdict, **measured out of the shipped font**
/// rather than guessed at. `Das Spiel endet unentschieden` sets 585 px in
/// `Faustina.ttf` at [`VERDICT_PT`] times [`super::SERIF_SCALE`], weight 600;
/// every other verdict is shorter, the nearest being `Team 2 gewinnt — deins`
/// at 466. (In `Inter.ttf`, which this replaced, the pair was 561 and 443 at
/// [`VERDICT_PT`] itself. The serif is the **wider** face here, by 4.4% — it
/// is set 10% larger to match Inter's x-height and does not give all of that
/// back in the advances, which is a cost of the change and not a benefit of
/// it.) Against it this sheet gives
/// 638 — 720 less two forty-pixel margins and its two one-pixel borders — so
/// the worst line has fifty pixels of air. If a verdict ever does wrap, widen
/// the sheet rather than shrink the type: a headline that changes size with
/// the sentence is a headline that says the sentence matters less.
const SHEET_W: f32 = 720.0;

/// The air the sheet leaves at each side of a window too narrow for
/// [`SHEET_W`]. Below about 816 logical pixels the sheet is the window less
/// this, and the verdict wraps to at most two lines rather than scaling down.
const SHEET_AIR: f32 = 96.0;

/// The verdict, in logical pixels.
///
/// Nothing else on this overlay is drawn above twenty, which is the point:
/// the largest thing the client ever has to say is the sentence that ends the
/// game.
const VERDICT_PT: f32 = 40.0;

/// Its line height, as a multiple of the size. A headline set at the body
/// text's leading looks like body text that was enlarged by accident.
const VERDICT_LEADING: f32 = 1.15;

/// The line under it.
const REASON_PT: f32 = 18.0;

/// The air between the verdict and the reason.
const REASON_GAP: f32 = 10.0;

/// The air between the sheet's prose and the way out of it.
const EXITS_GAP: f32 = 28.0;

/// How tall the exits row is held, whether or not anything has arrived in it.
///
/// Derived from the prompt slip's own answer: a 13 px label, `answer_node`'s
/// seven pixels of padding each way and its one-pixel border. Fixed rather
/// than grown from the content, because the buttons are another plugin's and
/// land a frame later, and a sheet that jumped as they arrived would be a
/// sheet that had not finished settling.
const EXITS_H: f32 = 32.0;

/// How far above its resting place the sheet starts.
///
/// It settles; it does not slide. Twelve pixels is a sheet coming to rest on
/// a table, and anything more is a panel flying in.
const SHEET_LIFT: f32 = 12.0;

/// The end screen's root. One per finished game, taken down on the way out.
#[derive(Component)]
pub(crate) struct FinishRoot;

/// The sheet itself, which is the one node that moves.
#[derive(Component)]
pub(crate) struct FinishSheet;

/// The row an embedding shell puts its exits in.
///
/// A marker rather than a callback because the two plugins do not know each
/// other and must not have to: the duel says *where* a way out belongs in this
/// composition, and whoever owns the screen after this duel says what the ways
/// out are.
#[derive(Component)]
pub(crate) struct FinishExits;

/// A node of this sheet, and the colour it rests at.
///
/// The resting colour has to be *remembered* rather than read back off the
/// node: one frame into the fade the node is carrying the faded value, and
/// taking that as the base is how a thing fades to nothing and stays there.
/// A node may carry two of these at once — the sheet has a fill and a border
/// — so all four channels are one component.
#[derive(Component, Clone, Copy)]
pub(crate) struct Settling {
    /// [`BackgroundColor`] at rest.
    fill: Option<Color>,
    /// [`BorderColor`] at rest.
    border: Option<Color>,
    /// [`TextColor`] at rest.
    ink: Option<Color>,
    /// [`ImageNode`]'s tint at rest — the parchment grain.
    grain: Option<Color>,
}

impl Settling {
    const NONE: Self = Self {
        fill: None,
        border: None,
        ink: None,
        grain: None,
    };

    /// A surface with an edge.
    const fn panel(fill: Color, border: Color) -> Self {
        Self {
            fill: Some(fill),
            border: Some(border),
            ..Self::NONE
        }
    }

    /// A line of text.
    const fn ink(ink: Color) -> Self {
        Self {
            ink: Some(ink),
            ..Self::NONE
        }
    }

    /// The grain laid over a sheet, which is an image and is tinted white.
    const fn grain() -> Self {
        Self {
            grain: Some(Color::WHITE),
            ..Self::NONE
        }
    }
}

/// Builds the screen, if there is a result to build it from.
///
/// `fonts` and `sheets` are optional for the reason every other drawing system
/// here takes them so: a headless test app runs `Startup` without a renderer,
/// and a screen that panicked rather than drawing nothing would make every
/// such test a test about the GPU.
pub(crate) fn spawn_finish(
    mut commands: Commands,
    duel: Res<crate::Duel>,
    settings: Res<crate::settings::ClientSettings>,
    fonts: Option<Res<UiFonts>>,
    sheets: Option<Res<UiSheets>>,
    windows: Query<&Window>,
) {
    let (Some(result), Some(fonts)) = (duel.ending(), fonts) else {
        return;
    };
    // The seat and its team, the same way the prompt bar used to find them: a
    // `Victor::Team` is answered by one comparison, and the seat's own team is
    // in the roster, which no view carries.
    let Some(statics) = duel.statics.as_ref() else {
        // No roster, so no way to say whose win this is. That is a game that
        // never really started, and a screen telling a player who never sat
        // down that they lost would be worse than no screen.
        return;
    };
    let seat = statics.your_seat;
    let team = duel.my_team();
    let lang = Lang::of(&settings.lang);
    let width = windows
        .iter()
        .next()
        .map_or(1280.0, |w| w.resolution.width());

    let root = commands
        .spawn((
            FinishRoot,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            // A **root**, so this is ordered against other roots and not
            // against `HudRoot`'s children: `ui_stack_system` sorts roots by
            // `(GlobalZIndex, ZIndex)` and then walks each subtree. Every
            // other root in this client sits at `GlobalZIndex(0)` — the seat
            // bars at -1 — so one is enough to stand over all of them, and it
            // says what it means instead of continuing the ladder of `ZIndex`
            // numbers that only ever order siblings.
            GlobalZIndex(1),
            Pickable::IGNORE,
        ))
        .id();

    // The same veil a zone dialog draws, from the same constructor and
    // painted by the same system: `tray::dim_the_table` now has a finished
    // game as its second reason to darken a table. The board stays visible
    // through it, because the board is the evidence for what the sheet says.
    let veil = super::tray::spawn_veil(&mut commands);
    commands.entity(root).add_child(veil);

    let sheet = commands
        .spawn((
            FinishSheet,
            Node {
                width: px(SHEET_W.min(width - SHEET_AIR)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect {
                    left: px(40),
                    right: px(40),
                    top: px(40),
                    bottom: px(32),
                },
                border: UiRect::all(px(1)),
                border_radius: sheet_radius(),
                ..default()
            },
            BackgroundColor(palette::PARCHMENT.with_alpha(0.0)),
            BorderColor::all(palette::PARCHMENT_EDGE.with_alpha(0.0)),
            sheet_shadow(),
            Settling::panel(palette::PARCHMENT, palette::PARCHMENT_EDGE),
            UiTransform::from_translation(Val2::px(0.0, SHEET_LIFT)),
            // Above the veil, which is its **sibling** under this root and
            // would otherwise be painted over it: `ui_stack_system` orders
            // siblings by `ZIndex`, and a node with none is a zero against
            // the veil's three. The ladder in `hud` says what that costs —
            // "the veil is the hinge: what is below it goes dark, what is
            // above it stays lit" — and this sheet was below it, drawn at
            // thirty per cent of the paper it is written on.
            ZIndex(Z_SHEET),
            Pickable::IGNORE,
        ))
        .id();
    if let Some(sheets) = sheets.as_deref() {
        commands
            .entity(sheet)
            .with_child((sheet_surface(sheets), Settling::grain()));
    }
    commands.entity(root).add_child(sheet);

    write_the_verdict(
        &mut commands,
        &fonts,
        sheet,
        Said {
            lang,
            result,
            seat,
            team,
        },
    );

    let exits = commands
        .spawn((
            FinishExits,
            Node {
                width: percent(100),
                height: px(EXITS_H),
                flex_direction: FlexDirection::Row,
                column_gap: px(BUTTON_GAP),
                justify_content: JustifyContent::Center,
                margin: UiRect::top(px(EXITS_GAP)),
                ..default()
            },
            // The row itself answers nothing; a button put in it brings its
            // own `Pickable`.
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(sheet).add_child(exits);
}

/// Everything the sheet has to say, and who is reading it.
///
/// One argument rather than four, because `spawn_finish` already passes a
/// `Commands`, a font set and the sheet to hang them on, and seven is where
/// this workspace stops counting parameters as readable.
struct Said<'a> {
    lang: Lang,
    result: &'a baylee_engine::win::GameResult,
    seat: baylee_core::ids::PlayerId,
    team: Option<u8>,
}

/// The verdict, and the line under it when there is one.
fn write_the_verdict(commands: &mut Commands, fonts: &UiFonts, sheet: Entity, said: Said) {
    let Said {
        lang,
        result,
        seat,
        team,
    } = said;
    let headline = commands
        .spawn((
            Text::new(verdict(lang, result, seat, team)),
            // The one place this client asks a face for a weight, and Faustina
            // is the one it can ask: Alegreya Sans ships as static cuts,
            // where `weight` reaches nothing at all.
            super::tf_serif(fonts, VERDICT_PT, 600),
            LineHeight::RelativeToFont(VERDICT_LEADING),
            // Opaque, unlike the slip's ink: `SLIP_INK` lets the grain through
            // at thirteen pixels, where it is a stroke with paper in it, and
            // at forty it would be a headline with holes in it.
            TextColor(palette::PARCHMENT_INK.with_alpha(0.0)),
            Settling::ink(palette::PARCHMENT_INK),
            TextLayout::justify(Justify::Center),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(sheet).add_child(headline);

    // A draw gets no second line at all, gap included — `ending_reason`
    // refuses it, because the only thing it could say is the verdict again.
    if let Some(reason) = ending_reason(lang, result) {
        let line = commands
            .spawn((
                Text::new(reason),
                tf_italic(fonts, REASON_PT),
                TextColor(palette::SLIP_SOFT.with_alpha(0.0)),
                Settling::ink(palette::SLIP_SOFT),
                TextLayout::justify(Justify::Center),
                Node {
                    margin: UiRect::top(px(REASON_GAP)),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(sheet).add_child(line);
    }
}

/// Every colour channel a settling node might carry.
///
/// A named shape rather than the tuple written out at the query, which clippy
/// is right to refuse: five `Option`s in a row say nothing about what they are
/// for.
type Painted = (
    &'static Settling,
    Option<&'static mut BackgroundColor>,
    Option<&'static mut BorderColor>,
    Option<&'static mut TextColor>,
    Option<&'static mut ImageNode>,
);

/// Settles the sheet as the veil rises.
///
/// It reads [`Veil::lit`] rather than easing anything of its own, which is
/// what makes the two one movement instead of two that could disagree: the
/// veil's rate, its resting point and its `reduce_motion` shortcut are the
/// sheet's as well, and nothing had to be told a transition is happening.
/// The same shape `sky::table_light` uses for the felt.
///
/// The exits are deliberately **not** settled. They belong to another plugin
/// and carry `ambience::Feel`, which owns their `BackgroundColor` from their
/// first frame; two systems writing that component would be two answers to
/// what colour a button is. Inside the fade or arriving after it, they simply
/// appear.
pub(crate) fn settle_the_sheet(
    veil: Res<Veil>,
    mut nodes: Query<Painted>,
    mut sheets: Query<&mut UiTransform, With<FinishSheet>>,
) {
    let lit = veil.lit.clamp(0.0, 1.0);
    for (settling, fill, border, ink, image) in &mut nodes {
        if let (Some(colour), Some(mut node)) = (settling.fill, fill) {
            node.0 = colour.with_alpha(colour.alpha() * lit);
        }
        if let (Some(colour), Some(mut node)) = (settling.border, border) {
            *node = BorderColor::all(colour.with_alpha(colour.alpha() * lit));
        }
        if let (Some(colour), Some(mut node)) = (settling.ink, ink) {
            node.0 = colour.with_alpha(colour.alpha() * lit);
        }
        if let (Some(colour), Some(mut node)) = (settling.grain, image) {
            node.color = colour.with_alpha(colour.alpha() * lit);
        }
    }
    for mut transform in &mut sheets {
        transform.translation = Val2::px(0.0, SHEET_LIFT * (1.0 - lit));
    }
}

/// Takes it down again — on the way back to the lobby, and on the way to
/// anything else.
pub(crate) fn despawn_finish(mut commands: Commands, roots: Query<Entity, With<FinishRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::PlayerId;
    use baylee_engine::win::{EndReason, GameResult, Victor};

    /// Fonts with no asset server behind them.
    ///
    /// A `Handle::default()` names no asset and draws no glyph, which is
    /// exactly right here: what is under test is the tree — which nodes exist,
    /// what they say and how they fade — and none of that is the GPU's.
    fn fonts() -> UiFonts {
        UiFonts {
            text: Handle::default(),
            medium: Handle::default(),
            italic: Handle::default(),
            medium_italic: Handle::default(),
            serif: Handle::default(),
            serif_italic: Handle::default(),
            icons: Handle::default(),
            mana: Handle::default(),
        }
    }

    fn statics(team: Option<u8>) -> baylee_view::GameStatic {
        baylee_view::GameStatic {
            view_version: baylee_view::VIEW_VERSION,
            game_id: "g".into(),
            your_seat: PlayerId::new(0),
            seats: vec![baylee_view::SeatIdentity {
                player: PlayerId::new(0),
                display_name: "me".into(),
                is_ai: false,
                away: false,
                team,
            }],
            prints: vec![],
        }
    }

    /// A duel that has just ended the given way, with the roster it needs.
    fn ended(result: GameResult, team: Option<u8>) -> crate::Duel {
        crate::Duel {
            statics: Some(statics(team)),
            interaction: Some(over(result)),
            ..crate::Duel::default()
        }
    }

    /// The engine's last word, as `Interaction` holds it.
    fn over(result: GameResult) -> baylee_client_core::Interaction {
        baylee_client_core::Interaction::new(
            baylee_engine::choice::Pending::GameOver(result),
            PlayerId::new(0),
        )
    }

    fn app_at(duel: crate::Duel) -> App {
        let mut app = App::new();
        app.insert_resource(duel)
            .insert_resource(crate::settings::ClientSettings::default())
            .insert_resource(fonts())
            .init_resource::<Veil>()
            .add_systems(Update, (spawn_finish, settle_the_sheet).chain());
        app.update();
        app
    }

    fn lines(app: &mut App) -> Vec<String> {
        let mut found = app.world_mut().query::<&Text>();
        found.iter(app.world()).map(|t| t.0.clone()).collect()
    }

    #[test]
    fn a_won_game_puts_its_verdict_and_its_reason_on_one_sheet() {
        let mut app = app_at(ended(
            GameResult {
                winner: Some(Victor::Player(PlayerId::new(0))),
                reason: EndReason::LastPlayerStanding,
            },
            None,
        ));
        let said = lines(&mut app);
        assert!(
            said.contains(&Phrase::YouWon.text(Lang::En).to_string()),
            "{said:?}"
        );
        assert!(
            said.contains(&Phrase::EndedLastPlayer.text(Lang::En).to_string()),
            "{said:?}"
        );
        let mut roots = app.world_mut().query_filtered::<Entity, With<FinishRoot>>();
        assert_eq!(roots.iter(app.world()).count(), 1);
        let mut exits = app
            .world_mut()
            .query_filtered::<Entity, With<FinishExits>>();
        assert_eq!(
            exits.iter(app.world()).count(),
            1,
            "the shell has nowhere to put a way out"
        );
    }

    /// The one ending whose reason would only say the verdict again.
    #[test]
    fn a_draw_is_one_line_and_not_two() {
        let mut app = app_at(ended(
            GameResult {
                winner: None,
                reason: EndReason::Draw,
            },
            None,
        ));
        let said = lines(&mut app);
        assert_eq!(
            said,
            vec![Phrase::TheGameIsADraw.text(Lang::En).to_string()],
            "a draw gets a verdict and nothing under it"
        );
    }

    /// A game with no roster cannot say whose win it is, so it says nothing.
    #[test]
    fn a_game_with_no_roster_draws_no_screen() {
        let mut app = app_at(crate::Duel {
            interaction: Some(over(GameResult {
                winner: Some(Victor::Player(PlayerId::new(0))),
                reason: EndReason::EffectWin,
            })),
            ..crate::Duel::default()
        });
        let mut roots = app.world_mut().query_filtered::<Entity, With<FinishRoot>>();
        assert_eq!(roots.iter(app.world()).count(), 0);
    }

    /// The sheet is spawned invisible and is painted by the veil's own number.
    ///
    /// Both halves matter. A sheet that arrived opaque would need no
    /// `Settling` at all, and a sheet painted from a fade of its own could
    /// drift from the veil it is supposed to arrive with — so the test moves
    /// `Veil::lit` by hand and reads the ink back out.
    #[test]
    fn the_sheet_settles_on_the_veils_own_number() {
        let mut app = app_at(ended(
            GameResult {
                winner: Some(Victor::Player(PlayerId::new(0))),
                reason: EndReason::LastPlayerStanding,
            },
            None,
        ));
        let ink = |app: &mut App| {
            let mut found = app.world_mut().query::<(&Settling, &TextColor)>();
            found
                .iter(app.world())
                .filter(|(s, _)| s.ink == Some(palette::PARCHMENT_INK))
                .map(|(_, c)| c.0.alpha())
                .next()
                .expect("a verdict")
        };
        let lift = |app: &mut App| {
            let mut found = app
                .world_mut()
                .query_filtered::<&UiTransform, With<FinishSheet>>();
            found.iter(app.world()).next().expect("a sheet").translation
        };
        assert!(ink(&mut app).abs() < 1e-6, "it is spawned clear");
        assert_eq!(lift(&mut app), Val2::px(0.0, SHEET_LIFT));

        app.world_mut().resource_mut::<Veil>().lit = 0.5;
        app.update();
        let half = ink(&mut app);
        assert!(
            (half - palette::PARCHMENT_INK.alpha() * 0.5).abs() < 1e-6,
            "half way up the veil the ink is half way up too: {half}"
        );
        assert_eq!(lift(&mut app), Val2::px(0.0, SHEET_LIFT * 0.5));

        app.world_mut().resource_mut::<Veil>().lit = 1.0;
        app.update();
        assert!((ink(&mut app) - palette::PARCHMENT_INK.alpha()).abs() < 1e-6);
        assert_eq!(lift(&mut app), Val2::px(0.0, 0.0), "it comes to rest");
    }

    /// The sheet stands over the veil it brought with it.
    ///
    /// It shipped under it. Both are children of the same root, the veil
    /// carries `Z_VEIL` and the sheet carried none, so the veil was painted
    /// over the whole screen — parchment `(224, 212, 176)` reached the window
    /// as `(125, 116, 94)` and brass `(201, 162, 39)` as `(117, 93, 24)`,
    /// each within a unit of what seventy per cent of `TABLE_VEIL` predicts.
    /// It read as a *dimmer* sheet rather than as a bug, which is exactly why
    /// the ladder is asserted and not merely written down.
    #[test]
    fn the_sheet_stands_over_the_veil_it_brought() {
        let mut app = app_at(ended(
            GameResult {
                winner: Some(Victor::Player(PlayerId::new(0))),
                reason: EndReason::LastPlayerStanding,
            },
            None,
        ));
        let mut sheets = app
            .world_mut()
            .query_filtered::<(&ZIndex, &ChildOf), With<FinishSheet>>();
        let (sheet_z, sheet_parent) = sheets.iter(app.world()).next().expect("a sheet");
        let (sheet_z, sheet_parent) = (sheet_z.0, sheet_parent.parent());
        let mut veils = app
            .world_mut()
            .query_filtered::<(&ZIndex, &ChildOf), With<TableVeil>>();
        let (veil_z, veil_parent) = veils.iter(app.world()).next().expect("a veil");
        let (veil_z, veil_parent) = (veil_z.0, veil_parent.parent());
        assert_eq!(
            sheet_parent, veil_parent,
            "the two are ordered against each other only as siblings"
        );
        assert!(
            sheet_z > veil_z,
            "the veil ({veil_z}) is painted over the sheet ({sheet_z})"
        );
    }
}
