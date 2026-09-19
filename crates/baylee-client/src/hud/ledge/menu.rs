//! The game menu: one button on the shelf's right end, and the panel it opens.
//!
//! The owner asked for it on 19.09.2026 — *"Aus den zwei Buttons rechts wird
//! ein Burger Menü. Es geht auf und dort ist ein schönes Menü (animationen zum
//! auf und zu gehen etc.), hier kommen noch mehr Buttons rein, aber erst Mal
//! die Zwei nur und eine Versionsanzeige+build des aktuellen Clients."* So
//! [`super::ways_out`]'s pair — offer a draw, concede — is now two rows in
//! here, the button that opens them is a burger, and under them stands the
//! one line that says which baylee this is.
//!
//! # Why the panel is retained and the button is not
//!
//! The button is drawn by [`super::sync_ledge`] like every other control on
//! the shelf, because it carries no state: it is a door, and a door looks the
//! same whichever side of it you are on. The **panel** cannot be, and the
//! reason is the same one [`super::pool`] gives one door along: the shelf's
//! columns are despawned and rebuilt whenever [`super::LedgeRevision`]
//! changes, and `concede_armed` is one of that revision's fields. A panel
//! living in the right column would therefore be despawned by the very click
//! that arms the concession — and respawned at `t = 0`, replaying its own
//! arrival, in the half-second the player has to find the second press in.
//! So it is a child of `HudRoot` that `sync_overlay`'s sweep passes over by
//! marker, the fourth of those after the drawer, the tray's strip and the
//! mana pool.
//!
//! # Where it stands, and what it is allowed to cover
//!
//! Hard against the window's right margin, growing **up** out of the shelf
//! with its bottom-right corner pinned — [`motion::from_bottom_right`], the
//! function `motion.rs` predicted would be wanted one day. Its bottom edge is
//! the strip's bottom edge, so an open menu covers [`super::tray`]'s archive
//! box entirely.
//!
//! That is allowed, and it is worth writing down why, because the sentence
//! next to `Z_TRAY` reads the other way at a glance. *"Demnach ist der Button
//! im Tray immer sichtbar"* was asked about a **maximised sheet** — a panel
//! that stands for as long as the player leaves it there, with no gesture of
//! their own holding it up. The owner's own gloss, 19.09.2026: *"Es bedeutet
//! nicht, dass dieser immer im Viewport sein muss, sondern dass er immer
//! irgend wie erreichbar sein soll … Er ist ein ganz gewöhnlicher Button wo
//! ein ausklapbares Menü (Submenü vom Burger) durchaus drüber darf."* A menu
//! the player is holding open, and closes with `Esc` or a second press of the
//! button that opened it, does not take the zones away; it stands in front of
//! them for as long as somebody is looking at it.
//!
//! # What the shelf gets back
//!
//! [`super::RIGHT_RESERVED`] was 222 px — the widest the pair ever gets — and
//! is now a burger and the margin. The width goes to the question in the
//! middle, which is the same trade the hand's tools made on the left when the
//! mana pool left that column.
//!
//! It also dissolves a workaround rather than merely shrinking one. The armed
//! concession used to be drawn **alone**, with the draw offer removed from
//! beside it, because the confirm wording is wider than both buttons together
//! and a grown button would have taken the press meant for the right end of
//! the question's last answer. In a column of fixed-width rows there is
//! nothing to overlap: the draw row stays where it is and goes
//! [`Weight::Dead`], which is also what makes the confirm row stay under the
//! pointer between the two presses instead of jumping 35 px up to fill the
//! gap.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;

/// How wide the panel is.
///
/// **Fixed**, and that is the decision rather than the number: a panel as wide
/// as its widest row would grow on the arming click, which moves the confirm
/// row out from under the pointer between press one and press two — the one
/// moment in this client where that matters most.
///
/// The number is a bound over two populations and is checked by
/// `the_panel_is_wide_enough_for_everything_it_can_ever_say`. The armed
/// concession is the widest label any *row* can show — `Aufgeben? Nochmal
/// drücken`, 174.9 px at [`LABEL_PT`] and 196.9 as a button — but it is not
/// what decides this number. The **version line** is: it is the one thing in
/// here that grows on its own, because the build number counts up with every
/// build made, and the worst shape it can take (a three-part version, six
/// digits of build, a ten-character commit and `-dirty`) is 40 characters at
/// [`VERSION_PT`], which is 228.8. Plus [`MENU_PAD_X`] either side and the
/// border, 262.8 — and the constant carries nine pixels over that.
///
/// Both halves of that were wrong at first and the test said so, which is the
/// argument for bounding it here rather than measuring once by hand: the
/// first draft sized the panel against the armed label alone and was six
/// pixels too narrow for a version string nobody in this repository has seen
/// yet.
const MENU_W: f32 = 272.0;

/// The air either side of a row. The drawer's number, because this is the
/// drawer's shape one level smaller.
const MENU_PAD_X: f32 = 16.0;

/// And above the first row and below the last line.
const MENU_PAD_Y: f32 = 10.0;

/// Between two rows.
const MENU_ROW_GAP: f32 = 7.0;

/// Between the last row and the rule, and between the rule and the version.
///
/// Wider than [`MENU_ROW_GAP`] on purpose: the rule is what says the line
/// under it is not a third button, and a rule crowded against a row reads as
/// that row's underline.
const MENU_RULE_GAP: f32 = 8.0;

/// The version line's size.
///
/// [`POOL_LABEL_PT`], which is the size this shelf already uses for the one
/// piece of text that never changes and is read once. Not bold, and not the
/// rows' [`LABEL_PT`]: it is the only thing in the panel that is not an
/// answer to anything.
const VERSION_PT: f32 = POOL_LABEL_PT;

/// The burger, in the shelf's row.
///
/// Square and a row button's own height, so it stands in the row rather than
/// beside it. [`super::tray`]'s buttons are four pixels shorter because they
/// stand inside a strip that has its own padding; this one has the column's.
pub(super) const BURGER: f32 = BUTTON_H;

/// The glyph's size in it, the tray button's.
const BURGER_PT: f32 = 12.0;

/// The retained panel.
#[derive(Component)]
pub struct MenuPanel;

/// How far the panel's own arrival or departure has run.
///
/// [`super::pool::StripZoom`]'s twin, and separate from it for the reason
/// `motion.rs` gives about every `t` in this HUD: the two are shown and
/// hidden by different systems reading different conditions, and one
/// component would have to be told which of them it belonged to.
///
/// It starts **closed and finished** — `t` at the end of a fold — so a panel
/// that has never been opened is hidden without anything having to run.
#[derive(Component)]
pub struct MenuZoom {
    t: f32,
    closing: bool,
}

impl Default for MenuZoom {
    fn default() -> Self {
        Self {
            t: 1.0,
            closing: true,
        }
    }
}

/// What the panel was last drawn from.
///
/// The fifth revision counter in this client, on the same argument as the
/// fourth: the panel changes when the menu is opened, when the concession is
/// armed and when this seat gains or loses priority, and the shelf changes
/// when the question does. One counter would have to lie about one of them.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
// Five bools, and they are five independent questions about one panel rather
// than a state that wants an enum: the menu can be open while the game is
// over, armed while a draw cannot be offered, and every pair of them occurs.
// Collapsing them would make the gate `==` compare something other than what
// the panel was built from, which is the one thing a revision counter must
// not do.
#[allow(clippy::struct_excessive_bools)]
pub struct MenuRevision {
    /// Whether the player has it open.
    open: bool,
    /// Whether the game has ended, which empties it of both ways out.
    over: bool,
    /// Whether the concession is waiting for its second press.
    armed: bool,
    /// Whether a draw may be offered at all (CR 104.4i).
    can_offer_draw: bool,
    /// The interface language, which is every label in here.
    lang: Option<Lang>,
}

/// Where the panel stands.
///
/// Its own function so a test can read it without an app, which is what
/// [`super::drawer::root_node`] and [`super::tray::root_node`] both have for
/// the same reason.
///
/// The bottom is the **strip's** bottom and not the strip's top: the panel
/// covers the strip rather than standing on it, so the two share an edge with
/// the shelf's lip and the join at the bottom is the one the strips already
/// make.
pub(in crate::hud) fn root_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        bottom: px(hand::HAND_ZONE_H - STRIP_LIP),
        right: px(EDGE),
        width: px(MENU_W),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        row_gap: px(MENU_ROW_GAP),
        padding: UiRect::axes(px(MENU_PAD_X), px(MENU_PAD_Y)),
        border: UiRect::all(px(1)),
        border_radius: BorderRadius::all(px(STRIP_R)),
        ..default()
    }
}

/// Spawns the panel, once, beside the shelf.
///
/// It is **pickable**, which is the one place it differs from both strips and
/// is not an oversight. A strip is exactly as wide as what is in it, so a
/// press on its padding is a press on nothing; this panel floats over the
/// table, so a press on its padding that fell through would reach whatever
/// card happens to be under it. The rows inside are pickable in their own
/// right.
pub(in crate::hud) fn spawn_menu_panel(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            MenuPanel,
            MenuZoom::default(),
            root_node(),
            BackgroundColor(palette::DIALOG),
            BorderColor::all(palette::DIALOG_LINE),
            ZIndex(Z_MENU),
            Visibility::Hidden,
        ))
        .id()
}

/// The burger itself, in the shelf's right column.
///
/// Three bars and they stay three bars. The obvious mark is the right one
/// here, which is not the answer this HUD usually gives — the zone browser's
/// archive box beat the layer-group because the layer-group was *taken* and
/// the box was the more exact of the two. Neither applies: nothing else on
/// this screen is three bars, a list of ways out of a game is a list, and the
/// owner said "Burger Menü".
///
/// **Not** an ✕ while it is open. [`glyph::CLOSE`] is the sheet head's cross
/// and means "this dialog's `Esc`" everywhere it is drawn; a burger that
/// turns into one is a glyph swap with no counterpart anywhere in this
/// interface, and it changes the mark a player has just aimed at. What says
/// the menu is open is the panel standing on the button, and the button's own
/// ground going to [`palette::DIALOG_LIT`] — which is exactly how
/// [`super::tray`]'s button says the same thing about the zone dialog.
pub(super) fn burger(commands: &mut Commands, fonts: &UiFonts, open: bool) -> Entity {
    let ground = if open {
        palette::DIALOG_LIT
    } else {
        palette::DIALOG
    };
    commands
        .spawn((
            Node {
                width: px(BURGER),
                height: px(BURGER),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(ground),
            BorderColor::all(palette::DIALOG_LINE),
            Button,
            Feel::new(ground),
            MenuButton {
                action: MenuAction::ToggleGameMenu,
            },
            children![(
                Text::new(glyph::BARS.to_string()),
                icon_tf(fonts, BURGER_PT),
                TextColor(palette::DIALOG_SOFT),
                Pickable::IGNORE,
            )],
        ))
        .id()
}

/// Fills the panel, or redraws it when what its rows say has changed.
///
/// Nothing here shows or hides the panel — [`grow_the_menu`] does both, at
/// the ends of the movement, for [`super::pool::sync_pool`]'s reason: a panel
/// taken off the screen on the frame it was dismissed never folds.
pub fn sync_menu(
    mut commands: Commands,
    duel: Res<Duel>,
    fonts: Res<UiFonts>,
    settings: Res<crate::settings::ClientSettings>,
    mut revision: ResMut<MenuRevision>,
    mut panel: Query<(Entity, Option<&Children>, &Visibility, &mut MenuZoom), With<MenuPanel>>,
) {
    let Ok((panel, standing, seen, mut fold)) = panel.single_mut() else {
        // No panel yet: the overlay has not built its root. The gate is left
        // untouched, so the next frame that has one builds on it.
        return;
    };
    let lang = Lang::of(&settings.lang);
    // A finished game empties it of both ways out, and since the version line
    // is all that would be left, it closes altogether. That is not tidiness:
    // `DuelSet::Input` does not run in `Finished`, so a menu left standing
    // under the end screen would be a panel that warms under the pointer and
    // answers nothing — the exact fault the pair in the corner had before
    // they came down to the shelf.
    let over = duel.ending().is_some();
    let next = MenuRevision {
        open: duel.game_menu && !over,
        over,
        armed: duel.concede_armed,
        can_offer_draw: duel.can_offer_draw(),
        lang: Some(lang),
    };
    // Whether the panel is *showing*, which is not whether it is visible: one
    // in the middle of folding away is still on the screen and is already
    // answered for. Reading `Visibility` alone would start the same close on
    // every frame until it finished, which is a fold that never gets past its
    // first frame. The same pair `sync_pool` and `sync_tray` both keep.
    let showing = *seen != Visibility::Hidden && !fold.closing;
    // And the tree beside the counter, for the reason `sync_tray_strip`
    // records: the overlay can take this node away and spawn a fresh one, and
    // a counter that still described the old panel would leave the new one
    // empty until something happened to change.
    let drawn = standing.is_some_and(|kids| !kids.is_empty());
    if *revision == next && showing == next.open && (drawn || !next.open) {
        return;
    }
    // Which way the panel is going. A change of direction restarts the
    // movement; a redraw in the same direction leaves it where it is, so a
    // concession armed inside an open panel does not reopen it.
    if next.open == fold.closing {
        fold.closing = !next.open;
        fold.t = 0.0;
    }
    *revision = next;

    if let Some(kids) = standing {
        for kid in kids {
            commands.entity(*kid).despawn();
        }
    }
    if !next.open {
        return;
    }

    // The draw offer keeps its place while the concession is armed rather
    // than being taken out of the column, which is what `ways_out` did with
    // it on the shelf. Two reasons and the second is the one that matters: a
    // draw cannot be offered in the middle of conceding, so `Dead` is what is
    // true; and a row that vanished would let the confirm row slide up under
    // a pointer that is about to press it.
    let offer_weight = if next.can_offer_draw && !next.armed {
        Weight::Secondary
    } else {
        Weight::Dead
    };
    let offer = answer(
        &mut commands,
        &fonts,
        Phrase::OfferADraw.text(lang),
        offer_weight,
        None,
    );
    if offer_weight != Weight::Dead {
        commands.entity(offer).insert(MenuButton {
            action: MenuAction::OfferDraw,
        });
    }
    commands.entity(panel).add_child(offer);

    let (words, weight) = if next.armed {
        (Phrase::ConcedeConfirm, Weight::Danger)
    } else {
        (Phrase::Concede, Weight::Secondary)
    };
    let concede = answer(&mut commands, &fonts, words.text(lang), weight, None);
    commands.entity(concede).insert(MenuButton {
        action: MenuAction::Concede,
    });
    commands.entity(panel).add_child(concede);

    for part in version_block(&mut commands, &fonts) {
        commands.entity(panel).add_child(part);
    }
}

/// The rule and the line under it: which baylee this is.
///
/// `baylee_build::short()`, the same string the lobby's header draws — and it
/// is the same string on purpose. What a bug report is worthless without is
/// one version, not two that were assembled separately and can disagree.
///
/// Three things set it apart from the rows above and no more: a rule, a
/// quieter ink, and no box at all. It is not pressable and does not warm.
fn version_block(commands: &mut Commands, fonts: &UiFonts) -> [Entity; 2] {
    let rule = commands
        .spawn((
            Node {
                height: px(1),
                margin: UiRect::top(px(MENU_RULE_GAP - MENU_ROW_GAP)),
                ..default()
            },
            BackgroundColor(palette::DIALOG_LINE),
            Pickable::IGNORE,
        ))
        .id();
    let line = commands
        .spawn((
            Text::new(baylee_build::short()),
            tf(fonts, VERSION_PT),
            TextColor(palette::DIALOG_SOFT),
            Node {
                margin: UiRect::top(px(MENU_RULE_GAP - MENU_ROW_GAP)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    [rule, line]
}

/// Opens the panel and shuts it, and is the only thing that shows or hides it.
///
/// [`super::pool::grow_the_pool`] with one corner changed, and the same three
/// claims: it is shown on the **first** frame of an arrival, so a panel is
/// never drawn full size before it has arrived; it is hidden on the **last**
/// frame of a fold, so it is never taken off the screen in the middle of its
/// own movement; and the corner that stays still is the one the button that
/// opened it is under.
pub fn grow_the_menu(
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    mut panels: Query<(&mut MenuZoom, &mut UiTransform, &mut Visibility), With<MenuPanel>>,
) {
    let still = prefs.all().reduce_motion;
    for (mut fold, mut transform, mut seen) in &mut panels {
        // At rest, either way, and an early continue rather than arithmetic
        // over the top of it: `Mut` writes on every deref, so a panel simply
        // standing there would mark itself changed on every frame.
        if fold.t >= 1.0 {
            continue;
        }
        let span = if fold.closing {
            motion::ZOOM_OUT
        } else {
            motion::ZOOM_IN
        };
        fold.t = motion::step(fold.t, span, time.delta_secs(), still);
        if fold.closing {
            if fold.t >= 1.0 {
                *seen = Visibility::Hidden;
                continue;
            }
        } else if *seen != Visibility::Inherited {
            *seen = Visibility::Inherited;
        }
        let scale = if fold.closing {
            motion::shutting(fold.t)
        } else {
            motion::opening(fold.t)
        };
        transform.scale = Vec2::splat(scale);
        transform.translation = motion::from_bottom_right(scale);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The panel is wide enough for everything it can ever say.
    ///
    /// Two populations and the second is the one that moves on its own: every
    /// row label in every language, and the version line, whose build number
    /// grows with every build made. So the version is bounded at a shape
    /// rather than at today's string — six digits of build number, a ten
    /// character commit and the `-dirty` suffix — and the real
    /// `baylee_build::short()` is held against that shape, which is what
    /// makes the bound a measurement instead of a guess.
    #[test]
    fn the_panel_is_wide_enough_for_everything_it_can_ever_say() {
        let inside = MENU_W - 2.0 * MENU_PAD_X - 2.0;
        for lang in [Lang::En, Lang::De] {
            for phrase in [Phrase::OfferADraw, Phrase::Concede, Phrase::ConcedeConfirm] {
                let label = phrase.text(lang);
                let row = crate::hud::text_width(label, LABEL_PT, true) + 2.0 * BUTTON_PAD_X + 2.0;
                assert!(
                    row <= inside,
                    "{lang:?} {label:?} sets {row:.1} and the panel holds {inside:.1}"
                );
            }
        }
        // The widest version string this scheme can produce: a three-part
        // version, a six-digit build number, a ten-character commit and the
        // dirty marker.
        let worst = "10.20.30+build.999999 (0123456789-dirty)";
        let line = crate::hud::text_width(worst, VERSION_PT, false);
        assert!(
            line <= inside,
            "a dirty build sets {line:.1} and the panel holds {inside:.1}"
        );
        let real = baylee_build::short();
        assert!(
            real.chars().count() <= worst.chars().count(),
            "the version is longer than the shape this was bounded at: {real:?}"
        );
    }

    /// The panel covers the strip rather than standing on it.
    ///
    /// Both are read out of the two `root_node`s rather than out of the
    /// constants, because what the claim is about is where two nodes are
    /// drawn: the strip and the panel share a bottom edge, and the panel is
    /// the taller of the two at every size it is ever drawn.
    #[test]
    fn the_panel_and_the_strip_stand_on_one_edge() {
        let strip = super::super::tray::root_node();
        let menu = root_node();
        assert_eq!(
            strip.bottom, menu.bottom,
            "the panel comes out of the shelf where the strip does"
        );
        assert_eq!(strip.right, menu.right, "and against the same margin");
        // That it stands *over* the strip is two constants compared, so it is
        // a `const _` beside `Z_MENU` itself rather than an assertion here
        // that could never fail at run time.
    }
}
