//! Donations at the front door (owner, 18.11.2026 and 25.11.2026): a button
//! at the top left of the sign-in screen, the month's goal as a thin bar
//! under it, and a sheet with the goal, what it pays for and the three
//! places to give, each with its code for a phone.
//!
//! The figures are Central's public `PublicFunding` answer, read without an
//! account (`baylee_account::funding_read`), as the website shows them; the
//! words and the goal are `client-core::funding`'s. What binds every place a
//! donation is asked for (`docs/legal.md` §"Donations"): it buys nothing, no
//! donor is named, Wizards never stands beside it (the fan content notice is
//! not part of this sheet), and it says the gift is not tax-deductible.
//!
//! The button draws the eye the way the kit's primaries do, by light and
//! colour, never by changing its size or its place: the kit's sheen on its
//! face, and its heart's colour breathing slowly (held still under reduced
//! motion).

use super::super::source;
#[allow(clippy::wildcard_imports)] // the lobby widget vocabulary
use super::super::*;
use super::FrontPress;
use crate::shellkit::controls::{self, Kit, Live, Weight};
use crate::shellkit::metrics::px_fixed;
use crate::shellkit::role::Role;
use crate::shellkit::{surfaces, tokens};
use baylee_account::funding::Funding;
use baylee_client_core::funding::{self, Platform};

/// The heart's colour: a warm rose that stands apart from the kit's teal
/// accent and gold without shouting.
pub(super) const ROSE: Color = Color::srgb(0.94, 0.45, 0.52);

/// How often the figures are read again while the front door stands.
const READ_EVERY_SECS: f64 = 600.0;

/// What the front door knows about the donations: the last figures read,
/// whether the sheet is up, and its codes.
#[derive(Default)]
pub(crate) struct Giving {
    /// The last figures Central gave; kept when a later read fails.
    pub(crate) funding: Option<Funding>,
    /// The sheet is up.
    pub(crate) open: bool,
    /// Each place's code, made when the sheet first opens.
    pub(crate) codes: Vec<(Platform, source::Code)>,
    /// A read in flight on its own thread.
    #[cfg(not(target_arch = "wasm32"))]
    reading: Option<std::sync::Arc<std::sync::Mutex<Option<Funding>>>>,
    /// When the last read was asked, in the app's seconds.
    asked_at: Option<f64>,
}

/// Reads the figures when the front door first stands and every ten
/// minutes while it does, on a thread of its own; takes the answer in.
///
/// A headless test asks nothing: it has no Central to reach.
pub(in crate::lobby) fn read(time: Res<Time>, mut state: ResMut<LobbyState>) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let arrived = state
            .giving
            .reading
            .as_ref()
            .and_then(|inbox| inbox.try_lock().ok().and_then(|mut slot| slot.take()));
        if let Some(funding) = arrived {
            state.giving.reading = None;
            if state.giving.funding.as_ref() != Some(&funding) {
                state.giving.funding = Some(funding);
            }
        }
        if cfg!(test) || state.giving.reading.is_some() || !showing(&state) {
            return;
        }
        let now = time.elapsed_secs_f64();
        if state
            .giving
            .asked_at
            .is_some_and(|at| now - at < READ_EVERY_SECS)
        {
            return;
        }
        let giving = &mut state.bypass_change_detection().giving;
        giving.asked_at = Some(now);
        let inbox: std::sync::Arc<std::sync::Mutex<Option<Funding>>> = std::sync::Arc::default();
        let writer = std::sync::Arc::clone(&inbox);
        let spawned = std::thread::Builder::new()
            .name("donation-figures".into())
            .spawn(move || {
                let read = crate::lobby::central::endpoint()
                    .ok()
                    .and_then(|endpoint| baylee_account::funding_read(&endpoint).ok());
                if let (Some(funding), Ok(mut slot)) = (read, writer.lock()) {
                    *slot = Some(funding);
                }
            });
        if spawned.is_ok() {
            giving.reading = Some(inbox);
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (time, state);
}

/// Whether the button stands: the sign-in screen, where the top left is free.
pub(super) fn showing(state: &LobbyState) -> bool {
    !state.lobby.offline() && !state.central_ready()
}

/// Makes the three codes once the sheet is up, before it is drawn.
pub(in crate::lobby) fn keep_the_codes(
    mut state: ResMut<LobbyState>,
    images: Option<ResMut<Assets<Image>>>,
) {
    if !state.giving.open || !state.giving.codes.is_empty() {
        return;
    }
    let Some(mut images) = images else {
        return;
    };
    let codes = Platform::ALL
        .into_iter()
        .filter_map(|platform| {
            let (image, side) = source::code_image(platform.address())?;
            Some((
                platform,
                source::Code {
                    url: platform.address().to_string(),
                    image: images.add(image),
                    side,
                },
            ))
        })
        .collect();
    state.giving.codes = codes;
}

/// What the month's figures say, if any platform has reported one: the
/// covered amount and the share of the goal.
fn month(state: &LobbyState) -> Option<u64> {
    state
        .giving
        .funding
        .as_ref()
        .and_then(funding::covered)
        .map(|(month, _)| month)
}

/// The button at the top left: the heart, its words, and under it the
/// month's bar and how much of the goal it covers.
pub(super) fn corner(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let lang = state.lobby.lang();
    let holder = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px_fixed(14.0),
                left: px_fixed(16.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                row_gap: kit.m.px(6.0),
                ..default()
            },
            GlobalZIndex(20),
            Pickable::IGNORE,
        ))
        .id();
    let button = button(commands, state, kit);
    commands.entity(holder).add_child(button);
    // The month's bar and its words stand only once a platform has said a
    // figure: no bar of nothing, and nothing appears later that moves the
    // button (the holder grows downwards, away from everything else).
    if let Some(month) = month(state) {
        let plate = month_plate(commands, kit, lang, month);
        commands.entity(holder).add_child(plate);
    }
    holder
}

/// The button itself: the rose heart and its words on the cluster's ground,
/// with the kit's sheen.
fn button(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let lang = state.lobby.lang();
    let h = px_fixed(kit.m.control * 0.78);
    let bg = Color::srgba(0.04, 0.08, 0.13, 0.56);
    let hot = Color::srgba(0.14, 0.09, 0.13, 0.86);
    let border = ROSE.with_alpha(0.45);
    let face = commands
        .spawn((
            Role::Button,
            Node {
                min_height: h,
                padding: UiRect::axes(kit.m.px(12.0), kit.m.px(4.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: kit.m.px(8.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(bg),
            BorderColor::all(border),
            surfaces::elevation_control(),
            crate::ambience::Feel::tinting_to(bg, hot),
            crate::ambience::FeelBorder {
                rest: border,
                hot: ROSE.with_alpha(0.75),
                press: ROSE,
            },
            crate::ambience::FeelShadow,
            crate::shellkit::sheen::Sheen,
        ))
        .id();
    let heart = commands
        .spawn((
            Text::new(crate::hud::glyph::HEART.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.small * 0.92),
            TextColor(ROSE),
            Heartbeat,
            Pickable::IGNORE,
        ))
        .id();
    let words = commands
        .spawn((
            Text::new(Phrase::DonateButton.text(lang)),
            crate::hud::tf_bold(kit.fonts, kit.m.small),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(face).add_children(&[heart, words]);
    controls::hit(
        commands,
        kit,
        face,
        (
            Press::Front(FrontPress::Donate),
            crate::shellkit::focus::Stop::new(super::keys::table_of(state).name, "donate"),
        ),
    )
}

/// Under the button, on a mist plate: the month's bar and what it covers.
fn month_plate(commands: &mut Commands, kit: Kit, lang: Lang, month: u64) -> Entity {
    let bar = crate::lobby::parts::meter(
        commands,
        kit,
        funding::share(month),
        Val::Percent(100.0),
        ROSE,
    );
    let caption = commands
        .spawn((
            Text::new(Phrase::DonateCovered.fill(
                lang,
                &[
                    &funding::euros(month, lang),
                    &funding::euros(funding::GOAL, lang),
                ],
            )),
            crate::hud::tf(kit.fonts, kit.m.small * 0.86),
            TextColor(tokens::MUTED),
            TextLayout::justify(Justify::Center),
            Pickable::IGNORE,
        ))
        .id();
    let under = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(4.0),
                padding: UiRect::axes(kit.m.px(8.0), kit.m.px(6.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(tokens::MIST),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(under).add_children(&[bar, caption]);
    under
}

/// The heart on the button: its colour breathes once every 2.4 seconds.
#[derive(Component)]
pub(in crate::lobby) struct Heartbeat;

/// Lets each [`Heartbeat`] breathe: a colour write on a frame that changes
/// it, nothing under reduced motion (the rose holds still).
pub(in crate::lobby) fn beat(
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    mut hearts: Query<&mut TextColor, With<Heartbeat>>,
) {
    let still = prefs.all().reduce_motion;
    for mut ink in &mut hearts {
        let next = if still {
            ROSE
        } else {
            let phase = (time.elapsed_secs() * std::f32::consts::TAU / 2.4).sin();
            let lift = 0.5 + 0.5 * phase;
            ROSE.mix(&Color::srgb(1.0, 0.72, 0.76), 0.55 * lift * lift)
        };
        if ink.0 != next {
            ink.0 = next;
        }
    }
}

/// The donation sheet: the goal and how much is covered, what it pays for,
/// the three places with their codes, and what a gift is and is not.
pub(super) fn sheet(commands: &mut Commands, root: Entity, state: &LobbyState, kit: Kit) {
    if !state.giving.open {
        return;
    }
    let lang = state.lobby.lang();
    let wide = kit.m.px(900.0);
    let dialog = surfaces::dialog(
        commands,
        kit,
        wide,
        ROSE.with_alpha(0.55),
        Press::Shared(SharedPress::PickerNothing),
    );
    commands
        .entity(dialog)
        // The solid glass, unlike the small community card: a sheet this
        // full is read, and the sign-in form must not show through its words.
        .insert((
            crate::shellkit::glass::ground(crate::shellkit::glass::Density::Solid),
            surfaces::SheetScroller,
            ScrollPosition::default(),
        ))
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.padding = UiRect::all(kit.m.px(22.0));
            node.row_gap = kit.m.px(16.0);
            node.max_height = Val::Percent(90.0);
            node.overflow = Overflow::scroll_y();
        });
    let scrim = surfaces::scrim(
        commands,
        kit,
        tokens::z::SHEET,
        Press::Front(FrontPress::CloseDonate),
        dialog,
    );
    let head = super::door::glass_head(
        commands,
        kit,
        crate::hud::glyph::HEART,
        ROSE,
        Phrase::DonateTitle.text(lang),
        Press::Front(FrontPress::CloseDonate),
    );
    let lead = paragraph(commands, kit, Phrase::DonateLead.text(lang), tokens::INK);
    commands.entity(dialog).add_children(&[head, lead]);
    // The goal and what it pays for side by side, equally wide.
    let pair = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                column_gap: kit.m.px(14.0),
                row_gap: kit.m.px(14.0),
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Stretch,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(dialog).add_child(pair);
    goal(commands, pair, state, kit);
    costs(commands, pair, kit, lang);
    ways(commands, dialog, state, kit);
    let nothing = paragraph(
        commands,
        kit,
        Phrase::DonateNothing.text(lang),
        tokens::MUTED,
    );
    let privacy = paragraph(
        commands,
        kit,
        Phrase::DonatePrivacy.text(lang),
        tokens::MUTED,
    );
    commands.entity(dialog).add_children(&[nothing, privacy]);
    commands.entity(root).add_child(scrim);
}

fn paragraph(commands: &mut Commands, kit: Kit, text: &str, ink: Color) -> Entity {
    commands
        .spawn((
            Text::new(text),
            crate::hud::tf(
                kit.fonts,
                if ink == tokens::MUTED {
                    kit.m.small
                } else {
                    kit.m.text
                },
            ),
            TextColor(ink),
            Node {
                width: Val::Percent(100.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

/// A caption in small capitals, the kit's way of naming a group.
fn caption(commands: &mut Commands, kit: Kit, text: &str) -> Entity {
    commands
        .spawn((
            Text::new(text.to_uppercase()),
            crate::hud::tf_bold(kit.fonts, kit.m.small),
            TextColor(tokens::MUTED),
            Pickable::IGNORE,
        ))
        .id()
}

/// The goal: its amount, the bar in a colour for each platform, the covered
/// amount and share, and who is counted. With no figure yet, one line says so.
fn goal(commands: &mut Commands, dialog: Entity, state: &LobbyState, kit: Kit) {
    let lang = state.lobby.lang();
    let block = inset(commands, kit);
    halve(commands, block);
    let title = caption(commands, kit, Phrase::DonateGoalTitle.text(lang));
    let amount = commands
        .spawn((
            Text::new(Phrase::DonateGoalAmount.fill(lang, &[&funding::euros(funding::GOAL, lang)])),
            crate::hud::tf_bold(kit.fonts, kit.m.head),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(block).add_children(&[title, amount]);
    let figures = state
        .giving
        .funding
        .as_ref()
        .and_then(|funding| funding::covered(funding).map(|covered| (funding, covered)));
    let Some((figures, (month, supporters))) = figures else {
        let none = paragraph(
            commands,
            kit,
            Phrase::DonateNoFigures.text(lang),
            tokens::MUTED,
        );
        commands.entity(block).add_child(none);
        commands.entity(dialog).add_child(block);
        return;
    };
    let track = split_bar(commands, kit, figures, month);
    let missing: Vec<&str> = Platform::ALL
        .into_iter()
        .filter(|platform| platform.monthly(figures).is_none())
        .map(Platform::name)
        .collect();
    let line = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                align_items: AlignItems::Baseline,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let have = commands
        .spawn((
            Text::new(Phrase::DonateCovered.fill(
                lang,
                &[
                    &funding::euros(month, lang),
                    &funding::euros(funding::GOAL, lang),
                ],
            )),
            crate::hud::tf_bold(kit.fonts, kit.m.text),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    let share = commands
        .spawn((
            Text::new(Phrase::DonatePercent.fill(lang, &[&funding::percent(month).to_string()])),
            crate::hud::tf_bold(kit.fonts, kit.m.text),
            TextColor(ROSE),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(line).add_children(&[have, share]);
    let legend = legend(commands, kit, lang, figures);
    let mut said = if month == 0 && supporters == 0 {
        Phrase::DonateFirst.text(lang).to_string()
    } else {
        Phrase::DonateSupporters.fill(lang, &[&supporters.to_string()])
    };
    if !missing.is_empty() {
        said.push(' ');
        said.push_str(&Phrase::DonateMissing.fill(lang, &[&missing.join(", ")]));
    }
    let counted = paragraph(commands, kit, &said, tokens::MUTED);
    commands
        .entity(block)
        .add_children(&[track, line, legend, counted]);
    commands.entity(dialog).add_child(block);
}

/// What a month costs, rounded: each cost against the goal.
fn costs(commands: &mut Commands, dialog: Entity, kit: Kit, lang: Lang) {
    let block = inset(commands, kit);
    halve(commands, block);
    let title = caption(commands, kit, Phrase::DonateCostsTitle.text(lang));
    commands.entity(block).add_child(title);
    for (phrase, euros) in [
        (Phrase::DonateCostAi, 430_u64),
        (Phrase::DonateCostServers, 100),
        (Phrase::DonateCostFees, 80),
        (Phrase::DonateCostArt, 50),
        (Phrase::DonateCostTools, 50),
    ] {
        let row = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    column_gap: kit.m.px(12.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let name = commands
            .spawn((
                Text::new(phrase.text(lang)),
                crate::hud::tf(kit.fonts, kit.m.small),
                TextColor(tokens::INK),
                Node {
                    width: kit.m.px(190.0),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        #[allow(clippy::cast_precision_loss)] // whole euros
        let share = (euros * 100) as f32 / funding::GOAL as f32;
        let bar = crate::lobby::parts::meter(commands, kit, share, Val::Auto, tokens::ACCENT);
        commands.entity(bar).entry::<Node>().and_modify(|mut n| {
            n.flex_grow = 1.0;
        });
        let amount = commands
            .spawn((
                Text::new(funding::euros(euros * 100, lang)),
                crate::hud::tf_bold(kit.fonts, kit.m.small),
                TextColor(tokens::INK),
                TextLayout::justify(Justify::Right),
                Node {
                    width: kit.m.px(64.0),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(row).add_children(&[name, bar, amount]);
        commands.entity(block).add_child(row);
    }
    commands.entity(dialog).add_child(block);
}

/// The three places to give, side by side: each one's sign, name, line,
/// code and button.
fn ways(commands: &mut Commands, dialog: Entity, state: &LobbyState, kit: Kit) {
    let lang = state.lobby.lang();
    let title = caption(commands, kit, Phrase::DonateWaysTitle.text(lang));
    let lead = paragraph(
        commands,
        kit,
        Phrase::DonateWaysLead.text(lang),
        tokens::MUTED,
    );
    let row = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                column_gap: kit.m.px(12.0),
                row_gap: kit.m.px(12.0),
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Stretch,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for platform in Platform::ALL {
        let card = inset(commands, kit);
        commands.entity(card).entry::<Node>().and_modify(|mut n| {
            n.width = Val::Auto;
            n.flex_grow = 1.0;
            n.flex_basis = Val::Px(0.0);
            n.min_width = Val::Px(180.0);
            n.align_items = AlignItems::Center;
        });
        let (glyph, lead) = match platform {
            Platform::GitHub => (crate::hud::glyph::HEART, Phrase::DonateGitHubLead),
            Platform::Liberapay => ('\u{f363}', Phrase::DonateLiberapayLead),
            Platform::Kofi => ('\u{f7b6}', Phrase::DonateKofiLead),
        };
        let name = crate::lobby::parts::marked_line(
            commands,
            kit,
            glyph,
            platform.name(),
            kit.m.text,
            colour(platform),
        );
        let words = paragraph(commands, kit, lead.text(lang), tokens::MUTED);
        commands
            .entity(words)
            .insert(TextLayout::justify(Justify::Center));
        let grow = crate::lobby::parts::grow(commands);
        commands.entity(card).add_children(&[name, words, grow]);
        if let Some((_, code)) = state.giving.codes.iter().find(|(p, _)| *p == platform) {
            let qr = super::door::qr_sized(commands, code, kit.m.px(96.0));
            commands.entity(card).add_child(qr);
        }
        let open = controls::button_with_icon(
            commands,
            kit,
            '\u{f35d}',
            &Phrase::DonateOpen.fill(lang, &[platform.name()]),
            Weight::Secondary,
            Live::Yes,
            None,
            Press::Front(FrontPress::OpenGiving(platform)),
        );
        commands
            .entity(open)
            .entry::<Node>()
            .and_modify(|mut n| n.width = Val::Percent(100.0));
        commands.entity(card).add_child(open);
        commands.entity(row).add_child(card);
    }
    commands.entity(dialog).add_children(&[title, lead, row]);
}

/// Each platform's colour on the bar, its legend and its card.
fn colour(platform: Platform) -> Color {
    match platform {
        Platform::GitHub => Color::srgb(0.72, 0.56, 0.96),
        Platform::Liberapay => Color::srgb(0.98, 0.79, 0.33),
        Platform::Kofi => Color::srgb(0.38, 0.76, 0.93),
    }
}

/// A block of the sheet: the control ground under a hairline.
fn inset(commands: &mut Commands, kit: Kit) -> Entity {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(8.0),
                padding: UiRect::all(kit.m.px(14.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL.with_alpha(0.45)),
            BorderColor::all(tokens::BORDER),
            Pickable::IGNORE,
        ))
        .id()
}

/// Gives a block of the goal's row an equal share of it, down to a width a
/// bar and its words still fit in (narrower, the row wraps).
fn halve(commands: &mut Commands, block: Entity) {
    commands.entity(block).entry::<Node>().and_modify(|mut n| {
        n.width = Val::Auto;
        n.flex_grow = 1.0;
        n.flex_basis = Val::Px(0.0);
        n.min_width = Val::Px(300.0);
    });
}

/// One bar of the goal, a part in its colour for each platform that reports,
/// as wide as its share of the goal (or of the month, where that is more).
fn split_bar(commands: &mut Commands, kit: Kit, figures: &Funding, month: u64) -> Entity {
    let track = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: kit.m.px(10.0),
                border_radius: BorderRadius::all(kit.m.px(5.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(tokens::CONTROL),
            Pickable::IGNORE,
        ))
        .id();
    let whole = month.max(funding::GOAL);
    for platform in Platform::ALL {
        let Some(monthly) = platform.monthly(figures) else {
            continue;
        };
        #[allow(clippy::cast_precision_loss)] // euro cents a month
        let part = (monthly.month as f32) / (whole as f32);
        if part <= 0.0 {
            continue;
        }
        let piece = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0 * part),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(colour(platform)),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(track).add_child(piece);
    }
    track
}

/// The bar's legend: each reporting platform's dot, name and month.
fn legend(commands: &mut Commands, kit: Kit, lang: Lang, figures: &Funding) -> Entity {
    let legend = commands
        .spawn((
            Node {
                flex_wrap: FlexWrap::Wrap,
                column_gap: kit.m.px(14.0),
                row_gap: kit.m.px(4.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for platform in Platform::ALL {
        let Some(monthly) = platform.monthly(figures) else {
            continue;
        };
        let key = crate::lobby::parts::glyph_line(
            commands,
            kit,
            '\u{f111}',
            &format!(
                "{} · {}",
                platform.name(),
                funding::euros(monthly.month, lang)
            ),
            colour(platform),
        );
        commands.entity(legend).add_child(key);
    }
    legend
}
