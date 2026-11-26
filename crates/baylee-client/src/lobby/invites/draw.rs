//! The invite sheet on screen. Its frame, its fields and their focus are
//! built once when it opens; each part that changes (the validity, the
//! email row, the action row, the new link, the list's head and its rows,
//! the question) is drawn again in place when what it shows changes, and
//! only then, so a resting sheet writes nothing and nothing in it moves
//! (owner: "kein Springen/Flackern/Layout-Verschieben").
//!
//! The form keeps its place while a new link stands over it (hidden, not
//! removed), so the list under it never shifts; the sheet has a fixed
//! height and its body scrolls. It opens and closes on the HUD's curves
//! (`hud::motion`), the new link pops in, "Copied" fades; `reduce_motion`
//! ends each movement on its first frame.

use super::{Action, CONFIRM, Field, Invites, Kind, ORDER};
use crate::hud::{UiFonts, glyph, icon_tf, motion, tf, tf_bold};
use crate::shellkit::focus::{ShellField, Stop};
use crate::shellkit::{
    InputClass, ShellMetrics, avatar,
    controls::{self, Kit, Live, Weight},
    metrics::px_fixed,
    qr, states,
    surfaces::{self, SheetHeader, SheetWidth},
    tokens,
};
use baylee_account::social::{FriendInviteStatus, FriendInviteSummary};
use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::social::ChatPhrase;
use baylee_client_core::social::invites::{self as model, Filter};
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;

/// The scrim the sheet stands on.
#[derive(Component)]
pub(super) struct Root;
/// The sheet itself, opening or closing.
#[derive(Component)]
pub(super) struct Sheet;
/// The question's scrim.
#[derive(Component)]
pub(super) struct ConfirmRoot;

/// Where a part of the sheet is drawn.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Slot {
    /// The form, kept in place under a new link.
    Form,
    Days,
    Email,
    Act,
    /// The new link, over the form.
    Made,
    Head,
    List,
}

/// A thing that opens on the HUD's curve and closes on it: `t` from 0 to
/// 1, and whether it is on its way out.
#[derive(Component, Default)]
pub(super) struct Pop {
    t: f32,
    leaving: bool,
    /// The scale it grows from and shrinks to.
    from: f32,
}

/// The line that says the link was copied.
#[derive(Component)]
pub(super) struct Copied;

/// What each part was last drawn from: `None` before it is drawn at all, so
/// a part drawn from nothing (no mail answer yet, no new link, no question)
/// is told apart from a part not drawn yet.
#[derive(Default)]
#[allow(clippy::option_option)] // not drawn yet, or drawn from nothing
pub(super) struct Drawn {
    shell: Option<(Lang, ShellMetrics)>,
    days: Option<u8>,
    email: Option<Option<bool>>,
    act: Option<(Option<Kind>, bool, Option<ChatPhrase>)>,
    made: Option<Option<u64>>,
    head: Option<(Filter, Option<(usize, usize)>)>,
    list: Option<ListKey>,
    confirm: Option<Option<([u8; 16], bool)>>,
}

/// What the list was drawn from: the invites (as their debug text, which
/// carries every field shown), the filter, whether reading failed, and the
/// minute, for "3 days left".
type ListKey = (u64, Filter, bool, bool, i64);

/// The sheet's height before the zoom: the form, the list's head and four
/// rows, and the list scrolls within it.
const HEIGHT: f32 = 700.0;

/// A short fingerprint of what a part shows, to tell whether it changed.
fn fingerprint(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one part after another
pub(super) fn draw(
    mut commands: Commands,
    sheet: Res<Invites>,
    state: Res<super::LobbyState>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    fonts: Res<UiFonts>,
    windows: Query<&Window>,
    input: Option<Res<InputClass>>,
    mut focus: Option<ResMut<InputFocus>>,
    mut drawn: Local<Drawn>,
    typed: Query<(&ShellField, &Field)>,
    roots: Query<Entity, With<Root>>,
    mut pops: Query<&mut Pop>,
    slots: Query<(Entity, &Slot)>,
    confirms: Query<Entity, With<ConfirmRoot>>,
    mut shown: Query<(&mut Visibility, &mut Node)>,
) {
    if !sheet.open() {
        if drawn.shell.take().is_some() {
            // The sheet and its question leave on the curve, then go.
            for root in roots.iter().chain(confirms.iter()) {
                if let Ok(mut pop) = pops.get_mut(root) {
                    pop.leaving = true;
                }
            }
            *drawn = Drawn::default();
        }
        return;
    }
    let lang = state.lobby.lang();
    let (width, height) = windows
        .iter()
        .next()
        .map_or((1024.0, 720.0), |w| (w.width(), w.height()));
    let metrics = super::super::header::kit_metrics(
        width,
        height,
        settings.as_deref().map_or_default(|s| s.text_size),
        input.as_deref().copied().unwrap_or_default(),
    );
    let kit = Kit {
        fonts: &fonts,
        m: metrics,
        german: lang == Lang::De,
    };
    // A new frame (the sheet opened, or the window or the language changed
    // under it): what was typed is typed into the new fields, and the parts
    // are drawn into its slots in this same frame, so it never stands empty.
    let mut fresh: Vec<(Slot, Entity)> = Vec::new();
    let reopened = drawn.shell.is_none();
    let typed = |which: Field| {
        typed
            .iter()
            .find(|(_, f)| **f == which)
            .filter(|_| !reopened)
            .map_or_else(String::new, |(field, _)| field.value.clone())
    };
    if drawn.shell != Some((lang, metrics)) {
        let kept = [typed(Field::Label), typed(Field::Note)];
        for root in roots.iter().chain(confirms.iter()) {
            commands.entity(root).try_despawn();
        }
        *drawn = Drawn::default();
        let (label, slots) = shell(&mut commands, kit, lang, &kept);
        fresh = slots;
        drawn.shell = Some((lang, metrics));
        if reopened && let Some(focus) = focus.as_deref_mut() {
            focus.set(label, FocusCause::Navigated);
        }
    }
    let slot = |which: Slot| {
        fresh
            .iter()
            .find(|(s, _)| *s == which)
            .map(|(_, e)| *e)
            .or_else(|| slots.iter().find(|(_, s)| **s == which).map(|(e, _)| e))
    };
    // The validity.
    if drawn.days != Some(sheet.days)
        && let Some(at) = slot(Slot::Days)
    {
        commands.entity(at).despawn_children();
        days(&mut commands, at, kit, lang, sheet.days);
        drawn.days = Some(sheet.days);
    }
    // The address row: a field where Central can mail, a quiet line where
    // it cannot (the same height, so nothing moves when it says).
    let mail = sheet.list.as_ref().map(|list| list.mail_available);
    if drawn.email != Some(mail)
        && let Some(at) = slot(Slot::Email)
    {
        commands.entity(at).despawn_children();
        let address = typed(Field::Email);
        email(&mut commands, at, kit, lang, mail != Some(false), &address);
        drawn.email = Some(mail);
    }
    // The action row: Create, held while a link is made or every slot is
    // taken, and why the last change was refused.
    let act_key = (sheet.busy(), sheet.full(), sheet.said);
    if drawn.act != Some(act_key)
        && let Some(at) = slot(Slot::Act)
    {
        commands.entity(at).despawn_children();
        act(&mut commands, at, kit, lang, &sheet);
        drawn.act = Some(act_key);
    }
    // The new link over the form, or the form.
    let made = sheet.made.as_ref().map(|m| m.serial);
    if drawn.made != Some(made)
        && let (Some(over), Some(form)) = (slot(Slot::Made), slot(Slot::Form))
    {
        commands.entity(over).despawn_children();
        if let Ok((mut seen, _)) = shown.get_mut(form) {
            seen.set_if_neq(if made.is_some() {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            });
        }
        if let Ok((_, mut node)) = shown.get_mut(over) {
            node.display = if made.is_some() {
                Display::Flex
            } else {
                Display::None
            };
        }
        if let Some(made) = sheet.made.as_ref() {
            made_link(&mut commands, over, kit, lang, made);
            commands.entity(over).insert(Pop {
                from: 0.96,
                ..default()
            });
            if let Some(focus) = focus.as_deref_mut() {
                focus.clear();
            }
        }
        drawn.made = Some(made);
    }
    // The list's head: its caption, how many are open, the filter.
    let head_key = (sheet.filter, sheet.slots());
    if drawn.head != Some(head_key)
        && let Some(at) = slot(Slot::Head)
    {
        commands.entity(at).despawn_children();
        head(&mut commands, at, kit, lang, &sheet);
        drawn.head = Some(head_key);
    }
    // The rows.
    let now = crate::social::now_ms() / 1000;
    let now = i64::try_from(now).unwrap_or(i64::MAX);
    let list_key: ListKey = (
        sheet
            .list
            .as_ref()
            .map_or(0, |list| fingerprint(&format!("{list:?}"))),
        sheet.filter,
        sheet.failed,
        sheet.list.is_none(),
        now / 60,
    );
    if drawn.list != Some(list_key)
        && let Some(at) = slot(Slot::List)
    {
        commands.entity(at).despawn_children();
        list(&mut commands, at, kit, lang, &sheet, now);
        drawn.list = Some(list_key);
    }
    // The question before an invite is withdrawn.
    let confirm_key = sheet
        .asking
        .map(|id| (id, sheet.busy() == Some(Kind::Withdraw)));
    if drawn.confirm != Some(confirm_key) {
        // Answered, the question leaves on the curve; held while Central
        // withdraws, it is drawn again where it stands.
        for root in &confirms {
            match pops.get_mut(root) {
                Ok(mut pop) if confirm_key.is_none() => pop.leaving = true,
                _ => commands.entity(root).try_despawn(),
            }
        }
        if let Some((id, busy)) = confirm_key {
            let invite = sheet
                .list
                .as_ref()
                .and_then(|l| l.invites.iter().find(|i| i.id == id));
            let opened = drawn.confirm.flatten().is_none();
            question(&mut commands, kit, lang, invite, busy, opened);
        }
        drawn.confirm = Some(confirm_key);
    }
}

/// The sheet's frame: the scrim, the sheet with its head, the name and
/// greeting fields with what was `kept` in them, and the empty slots its
/// other parts are drawn into. Answers the first field, which takes the
/// keys, and the slots.
#[allow(clippy::too_many_lines)] // the frame, one slot after another
fn shell(
    commands: &mut Commands,
    kit: Kit,
    lang: Lang,
    kept: &[String; 2],
) -> (Entity, Vec<(Slot, Entity)>) {
    let column = |commands: &mut Commands, slot: Slot, gap: f32| {
        commands
            .spawn((
                slot,
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: kit.m.px(gap),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id()
    };
    // The form, and over it the new link.
    let form = column(commands, Slot::Form, 10.0);
    let caption = caption(commands, kit, ChatPhrase::InviteNewCaption.text(lang));
    let days = commands
        .spawn((
            Slot::Days,
            Node {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: kit.m.px(10.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let label = field(
        commands,
        kit,
        (Field::Label, "label"),
        glyph::TAG,
        ChatPhrase::InviteLabelHint.text(lang),
        &kept[0],
    );
    let note = field(
        commands,
        kit,
        (Field::Note, "note"),
        glyph::QUOTE,
        ChatPhrase::InviteNoteHint.text(lang),
        &kept[1],
    );
    let email = column(commands, Slot::Email, 0.0);
    let act = column(commands, Slot::Act, 6.0);
    commands
        .entity(form)
        .add_children(&[caption, days, label, note, email, act]);
    let made = commands
        .spawn((
            Slot::Made,
            Node {
                position_type: PositionType::Absolute,
                left: px_fixed(0.0),
                right: px_fixed(0.0),
                top: px_fixed(0.0),
                bottom: px_fixed(0.0),
                display: Display::None,
                ..default()
            },
            UiTransform::default(),
            Pickable::IGNORE,
        ))
        .id();
    let top = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .add_children(&[form, made])
        .id();
    let rule = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: px_fixed(1.0),
                margin: UiRect::vertical(kit.m.px(4.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(tokens::BORDER),
            Pickable::IGNORE,
        ))
        .id();
    let head = commands
        .spawn((
            Slot::Head,
            Node {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                column_gap: kit.m.px(10.0),
                row_gap: kit.m.px(6.0),
                flex_wrap: FlexWrap::Wrap,
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let rows = column(commands, Slot::List, 0.0);
    let header = SheetHeader::new(ChatPhrase::InvitesTitle.text(lang))
        .icon(glyph::USER_PLUS)
        .subtitle(ChatPhrase::InvitesLead.text(lang))
        .close((Action::Close, Stop::new(ORDER.name, "close")));
    let surface = surfaces::sheet_box_fixed(
        commands,
        kit,
        SheetWidth::Small,
        header,
        &[top, rule, head, rows],
        &[],
    );
    let tall = kit.m.px(HEIGHT);
    commands
        .entity(surface)
        .insert((
            Sheet,
            Action::Nothing,
            UiTransform::from_scale(Vec2::splat(motion::ZOOM_FROM)),
        ))
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.height = tall;
        });
    let root = surfaces::scrim(commands, kit, tokens::z::SHEET + 1, Action::Close, surface);
    commands.entity(root).insert((
        Root,
        Pop {
            from: motion::ZOOM_FROM,
            ..default()
        },
        BackgroundColor(Color::NONE),
    ));
    let slots = vec![
        (Slot::Form, form),
        (Slot::Days, days),
        (Slot::Email, email),
        (Slot::Act, act),
        (Slot::Made, made),
        (Slot::Head, head),
        (Slot::List, rows),
    ];
    (label, slots)
}

/// A group's name in small capitals, as the kit's sheets name them.
fn caption(commands: &mut Commands, kit: Kit, text: &str) -> Entity {
    commands
        .spawn((
            Text::new(text.to_uppercase()),
            tf_bold(kit.fonts, kit.m.small),
            TextColor(tokens::MUTED),
            Pickable::IGNORE,
        ))
        .id()
}

/// One of the form's fields, which the kit edits, holding `value`.
fn field(
    commands: &mut Commands,
    kit: Kit,
    (which, stop): (Field, &'static str),
    glyph: char,
    hint: &str,
    value: &str,
) -> Entity {
    controls::field(
        commands,
        kit,
        glyph,
        value,
        hint,
        (
            which,
            ShellField::new(value, hint),
            Stop::new(ORDER.name, stop),
        ),
    )
}

/// "Valid for" and the four validities.
fn days(commands: &mut Commands, at: Entity, kit: Kit, lang: Lang, chosen: u8) {
    let words = controls::label(
        commands,
        kit,
        ChatPhrase::InviteValidFor.text(lang),
        kit.m.text,
        tokens::INK,
    );
    let labels: Vec<String> = model::DAYS
        .iter()
        .map(|d| ChatPhrase::InviteDays.text_count(lang, usize::from(*d)))
        .collect();
    let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
    let picked = model::DAYS.iter().position(|d| *d == chosen).unwrap_or(0);
    let choice = controls::segmented_compact(commands, kit, &labels, picked, |i| {
        (
            Action::Days(model::DAYS[i]),
            Stop::item(ORDER.name, "days", u8::try_from(i).unwrap_or(0)),
        )
    });
    commands.entity(at).add_children(&[words, choice]);
}

/// The address the invite is mailed to, or, where Central cannot mail, a
/// quiet line in the field's place.
fn email(commands: &mut Commands, at: Entity, kit: Kit, lang: Lang, can: bool, value: &str) {
    let row = if can {
        field(
            commands,
            kit,
            (Field::Email, "email"),
            glyph::ENVELOPE,
            ChatPhrase::InviteEmailHint.text(lang),
            value,
        )
    } else {
        let row = controls::field(
            commands,
            kit,
            glyph::ENVELOPE,
            "",
            ChatPhrase::InviteMailUnavailable.text(lang),
            (),
        );
        commands
            .entity(row)
            .insert(BackgroundColor(tokens::CONTROL.with_alpha(0.4)));
        row
    };
    commands.entity(at).add_child(row);
}

/// Create, as wide as the form, and under it why the last change was
/// refused (a line that is always there, so a refusal moves nothing).
fn act(commands: &mut Commands, at: Entity, kit: Kit, lang: Lang, sheet: &Invites) {
    let making = sheet.busy() == Some(Kind::Create);
    let full = sheet
        .slots()
        .filter(|(open, most)| open >= most)
        .map(|(_, most)| ChatPhrase::InviteSlotsFull.text_count(lang, most));
    let (text, live) = if making {
        (
            ChatPhrase::InviteCreating.text(lang),
            Live::No(ChatPhrase::InviteCreating.text(lang)),
        )
    } else if let Some(why) = full.as_deref() {
        (ChatPhrase::InviteCreate.text(lang), Live::No(why))
    } else {
        (ChatPhrase::InviteCreate.text(lang), Live::Yes)
    };
    let create = controls::button_with_icon(
        commands,
        kit,
        glyph::LINK,
        text,
        Weight::Primary,
        live,
        None,
        (Action::Create, Stop::new(ORDER.name, "create")),
    );
    commands
        .entity(create)
        .entry::<Node>()
        .and_modify(|mut node| node.width = Val::Percent(100.0));
    let (said, ink) = match (sheet.said, full.as_deref()) {
        (Some(said), _) => (said.text(lang).to_owned(), tokens::DANGER),
        (None, Some(full)) => (full.to_owned(), tokens::MUTED),
        (None, None) => (String::new(), tokens::MUTED),
    };
    let line = commands
        .spawn((
            Text::new(said),
            tf(kit.fonts, kit.m.small),
            TextColor(ink),
            Node {
                min_height: kit.m.px(18.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(at).add_children(&[create, line]);
}

/// The new link: its code for a phone, the address in full, Copy and Make
/// another, "Copied" while it stands, until when it holds, that it is shown
/// once, and whether the mail went out.
#[allow(clippy::too_many_lines)] // one line of the card after another
fn made_link(commands: &mut Commands, at: Entity, kit: Kit, lang: Lang, made: &super::Made) {
    let row = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                column_gap: kit.m.px(16.0),
                align_items: AlignItems::FlexStart,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    if let Some((image, _)) = made.code.as_ref() {
        let code = qr::framed(commands, image.clone(), kit.m.px(124.0));
        let scan = commands
            .spawn((
                Text::new(ChatPhrase::InviteScanHint.text(lang)),
                tf(kit.fonts, kit.m.small),
                TextColor(tokens::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        let column = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: kit.m.px(6.0),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .add_children(&[code, scan])
            .id();
        commands.entity(row).add_child(column);
    }
    let words = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(8.0),
                flex_grow: 1.0,
                flex_basis: px_fixed(0.0),
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let title = caption(commands, kit, ChatPhrase::InviteReadyCaption.text(lang));
    // The address in full, as a phone reads it from the code.
    let address = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(kit.m.px(10.0), kit.m.px(8.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL),
            BorderColor::all(tokens::ACCENT.with_alpha(0.45)),
            Pickable::IGNORE,
        ))
        .with_child((
            Text::new(made.url.as_str()),
            tf(kit.fonts, kit.m.small),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    let copy = controls::button_compact(
        commands,
        kit,
        Some(glyph::COPY),
        ChatPhrase::InviteCopyLink.text(lang),
        Weight::Primary,
        Live::Yes,
        (Action::Copy, Stop::new(ORDER.name, "copy")),
    );
    let again = controls::button_compact(
        commands,
        kit,
        Some(glyph::PLUS),
        ChatPhrase::InviteAnother.text(lang),
        Weight::Secondary,
        Live::Yes,
        (Action::Another, Stop::new(ORDER.name, "another")),
    );
    let buttons = commands
        .spawn((
            Node {
                column_gap: kit.m.px(8.0),
                row_gap: kit.m.px(6.0),
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .add_children(&[copy, again])
        .id();
    // Always there, clear until a copy: it fades in and out where it
    // stands, and nothing moves.
    let copied = crate::lobby::parts::marked_line(
        commands,
        kit,
        glyph::CHECK,
        ChatPhrase::InviteCopied.text(lang),
        kit.m.small,
        tokens::ACCENT.with_alpha(0.0),
    );
    commands.entity(copied).insert(Copied);
    let until = model::when(made.expires_at, super::link::local_offset(), lang);
    let valid = line(
        commands,
        kit,
        &ChatPhrase::InviteValidUntil.fill(lang, &[&until]),
        tokens::MUTED,
    );
    let once = line(
        commands,
        kit,
        ChatPhrase::InviteShownOnce.text(lang),
        tokens::GOLD,
    );
    commands
        .entity(words)
        .add_children(&[title, address, buttons, copied, valid, once]);
    if let Some((address, sent)) = made.mail.as_ref() {
        let said = if *sent {
            ChatPhrase::InviteMailed.fill(lang, &[address])
        } else {
            ChatPhrase::InviteMailNotSent.text(lang).to_owned()
        };
        let mail = crate::lobby::parts::marked_line(
            commands,
            kit,
            glyph::ENVELOPE,
            &said,
            kit.m.small,
            if *sent { tokens::INK } else { tokens::DANGER },
        );
        commands.entity(words).add_child(mail);
    }
    commands.entity(row).add_child(words);
    commands.entity(at).add_child(row);
}

/// A line of the new link's card, wrapping.
fn line(commands: &mut Commands, kit: Kit, text: &str, ink: Color) -> Entity {
    let e = surfaces::prose(commands, kit, text, true);
    commands.entity(e).insert(TextColor(ink));
    e
}

/// The list's head: its caption, how many are open of how many may be,
/// and the filter.
fn head(commands: &mut Commands, at: Entity, kit: Kit, lang: Lang, sheet: &Invites) {
    let title = caption(commands, kit, ChatPhrase::InvitesListCaption.text(lang));
    let slots = sheet.slots().map_or_else(String::new, |(open, most)| {
        ChatPhrase::InviteSlots.fill(lang, &[&open.to_string(), &most.to_string()])
    });
    let count = commands
        .spawn((
            Text::new(slots),
            tf(kit.fonts, kit.m.small),
            TextColor(if sheet.full() {
                tokens::GOLD
            } else {
                tokens::MUTED
            }),
            Node {
                flex_grow: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let labels: Vec<&str> = Filter::ALL.iter().map(|f| f.label(lang)).collect();
    let picked = Filter::ALL
        .iter()
        .position(|f| *f == sheet.filter)
        .unwrap_or(0);
    let filter = controls::segmented_compact(commands, kit, &labels, picked, |i| {
        (
            Action::Filter(Filter::ALL[i]),
            Stop::item(ORDER.name, "filter", u8::try_from(i).unwrap_or(0)),
        )
    });
    commands.entity(at).add_children(&[title, count, filter]);
}

/// The invites the filter shows, newest first; a skeleton until the first
/// list came, a line with Retry when it could not be read, and a picture
/// with a sentence where there is nothing to show.
fn list(commands: &mut Commands, at: Entity, kit: Kit, lang: Lang, sheet: &Invites, now: i64) {
    let Some(listed) = sheet.list.as_ref() else {
        let shown = if sheet.failed {
            let retry = controls::button_compact(
                commands,
                kit,
                Some(glyph::REFRESH),
                ChatPhrase::RetryButton.text(lang),
                Weight::Secondary,
                Live::Yes,
                Action::Retry,
            );
            states::error_line(commands, kit, ChatPhrase::InvitesFailed.text(lang), retry)
        } else {
            states::skeleton(commands, kit, 3)
        };
        commands.entity(at).add_child(shown);
        return;
    };
    let offset = super::link::local_offset();
    let shown: Vec<&FriendInviteSummary> = listed
        .invites
        .iter()
        .filter(|invite| sheet.filter.shows(invite.status))
        .collect();
    if shown.is_empty() {
        let sentence = match sheet.filter {
            _ if listed.invites.is_empty() => ChatPhrase::InvitesEmpty,
            Filter::Open => ChatPhrase::InvitesEmptyOpen,
            Filter::Taken | Filter::All => ChatPhrase::InvitesEmptyTaken,
        };
        let glyph = if listed.invites.is_empty() {
            glyph::USER_PLUS
        } else {
            glyph::LINK
        };
        let empty = states::empty(commands, kit, glyph, sentence.text(lang), None);
        commands.entity(at).add_child(empty);
        return;
    }
    let mut withdraws: u8 = 0;
    for invite in shown {
        let row = row(commands, kit, lang, invite, now, offset, &mut withdraws);
        commands.entity(at).add_child(row);
    }
}

/// An invite's mark: the disc of who took it up, else a round plate with a
/// link (open), a struck circle (withdrawn) or an hourglass (expired).
fn mark(commands: &mut Commands, kit: Kit, invite: &FriendInviteSummary, open: bool) -> Entity {
    if let (Some(person), FriendInviteStatus::Redeemed) = (&invite.redeemed_by, invite.status) {
        return avatar::disc(
            commands,
            kit.fonts,
            &model::handle(person),
            kit.m.scaled(34.0),
        );
    }
    let (glyph, ink, ground) = if open {
        (glyph::LINK, tokens::ACCENT, tokens::ACCENT.with_alpha(0.14))
    } else if invite.status == FriendInviteStatus::Revoked {
        (glyph::BAN, tokens::MUTED, tokens::SKELETON)
    } else {
        (glyph::HOURGLASS, tokens::MUTED, tokens::SKELETON)
    };
    let size = kit.m.px(34.0);
    commands
        .spawn((
            Node {
                width: size,
                height: size,
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(Val::Percent(50.0)),
                ..default()
            },
            BackgroundColor(ground),
            BorderColor::all(ink.with_alpha(0.35)),
            Pickable::IGNORE,
        ))
        .with_child((
            Text::new(glyph.to_string()),
            icon_tf(kit.fonts, kit.m.small),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id()
}

/// One invite: its mark, its name and state, and Withdraw while it is open.
fn row(
    commands: &mut Commands,
    kit: Kit,
    lang: Lang,
    invite: &FriendInviteSummary,
    now: i64,
    offset: i32,
    withdraws: &mut u8,
) -> Entity {
    let open = invite.status == FriendInviteStatus::Active && invite.expires_at > now;
    let line = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                column_gap: kit.m.px(12.0),
                padding: UiRect::vertical(kit.m.px(9.0)),
                border: UiRect::bottom(px_fixed(1.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BorderColor::all(tokens::BORDER.with_alpha(0.5)),
            Pickable::IGNORE,
        ))
        .id();
    let mark = mark(commands, kit, invite, open);
    let mut state = model::state_words(invite, now, offset, lang);
    if let Some(address) = invite.invited_email.as_deref() {
        state = format!("{state} · {address}");
    }
    let words = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(2.0),
                flex_grow: 1.0,
                flex_basis: px_fixed(0.0),
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|words| {
            words.spawn((
                Text::new(model::title(invite, offset, lang)),
                tf(kit.fonts, kit.m.text),
                TextColor(if open { tokens::INK } else { tokens::MUTED }),
                Pickable::IGNORE,
            ));
            words.spawn((
                Text::new(state),
                tf(kit.fonts, kit.m.small),
                TextColor(if open { tokens::ACCENT } else { tokens::MUTED }),
                Pickable::IGNORE,
            ));
        })
        .id();
    commands.entity(line).add_children(&[mark, words]);
    if open {
        let withdraw = controls::button_compact(
            commands,
            kit,
            Some(glyph::TRASH),
            ChatPhrase::InviteRevoke.text(lang),
            Weight::Ghost,
            Live::Yes,
            (
                Action::Withdraw(invite.id),
                Stop::item(ORDER.name, "withdraw", *withdraws),
            ),
        );
        *withdraws = withdraws.saturating_add(1);
        commands.entity(line).add_child(withdraw);
    }
    line
}

/// "Withdraw this invite?": the invite's name, what withdrawing does,
/// Cancel and Withdraw (held while Central withdraws it). `Esc` and a press
/// beside it are Cancel; no key says yes but Enter on Withdraw itself.
fn question(
    commands: &mut Commands,
    kit: Kit,
    lang: Lang,
    invite: Option<&FriendInviteSummary>,
    busy: bool,
    opened: bool,
) {
    let dialog = surfaces::dialog(
        commands,
        kit,
        kit.m.px(440.0),
        tokens::DANGER.with_alpha(0.5),
        Action::Nothing,
    );
    let title = invite.map(|i| model::title(i, super::link::local_offset(), lang));
    let mut header = SheetHeader::new(ChatPhrase::InviteRevokeQuestion.text(lang))
        .icon_colored(glyph::TRASH, tokens::DANGER);
    if let Some(title) = title.as_deref() {
        header = header.subtitle(title);
    }
    let head = surfaces::dialog_header(commands, kit, header);
    let hint = surfaces::prose(
        commands,
        kit,
        ChatPhrase::InviteRevokeHint.text(lang),
        false,
    );
    let body = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(14.0),
                padding: UiRect::all(px_fixed(kit.m.pad)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let cancel = controls::button_compact(
        commands,
        kit,
        None,
        Phrase::ActCancel.text(lang),
        Weight::Secondary,
        Live::Yes,
        (Action::Cancel, Stop::new(CONFIRM.name, "cancel")),
    );
    let withdraw = controls::button_compact(
        commands,
        kit,
        Some(glyph::TRASH),
        ChatPhrase::InviteRevoke.text(lang),
        Weight::Danger,
        if busy {
            Live::No(Phrase::VeilTalking.text(lang))
        } else {
            Live::Yes
        },
        (Action::Confirm, Stop::new(CONFIRM.name, "withdraw")),
    );
    let buttons = surfaces::dialog_button_row(commands, kit, &[cancel, withdraw]);
    commands.entity(body).add_children(&[hint, buttons]);
    commands.entity(dialog).add_children(&[head, body]);
    let from = 0.94;
    commands
        .entity(dialog)
        .insert(UiTransform::from_scale(Vec2::splat(if opened {
            from
        } else {
            1.0
        })));
    let root = surfaces::scrim(commands, kit, tokens::z::SHEET + 2, Action::Cancel, dialog);
    commands.entity(root).insert((
        ConfirmRoot,
        Pop {
            t: if opened { 0.0 } else { 1.0 },
            from,
            leaving: false,
        },
        BackgroundColor(if opened { Color::NONE } else { tokens::SCRIM }),
    ));
}

/// Moves what opens and closes, fades "Copied", and lets a sheet that left
/// go. Writes nothing once everything rests.
#[allow(clippy::type_complexity, clippy::too_many_arguments)] // a Bevy system: the movers and what they move
pub(super) fn animate(
    mut commands: Commands,
    time: Option<Res<Time>>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut sheet: ResMut<Invites>,
    mut pops: Query<(
        Entity,
        &mut Pop,
        &Children,
        Option<&mut BackgroundColor>,
        Has<Slot>,
    )>,
    mut scales: Query<&mut UiTransform>,
    copied: Query<&Children, With<Copied>>,
    mut inks: Query<&mut TextColor>,
) {
    let delta = time.map_or(0.016, |t| t.delta_secs());
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    for (entity, mut pop, children, ground, inner) in &mut pops {
        let resting = if pop.leaving { false } else { pop.t >= 1.0 };
        if resting {
            continue;
        }
        let span = if pop.leaving {
            motion::ZOOM_OUT
        } else {
            motion::ZOOM_IN
        };
        if pop.leaving {
            // Leaving runs `t` back down to nothing.
            pop.t = 1.0 - motion::step(1.0 - pop.t, span, delta, still);
        } else {
            pop.t = motion::step(pop.t, span, delta, still);
        }
        let shown = if pop.leaving {
            pop.t
        } else {
            1.0 - (1.0 - pop.t).powi(2)
        };
        let scale = if pop.leaving {
            1.0 - (1.0 - pop.from) * (1.0 - pop.t) * (1.0 - pop.t)
        } else {
            pop.from + (1.0 - pop.from) * motion::pop(pop.t)
        };
        // A slot pops itself; a scrim fades and pops the sheet it holds.
        let target = if inner {
            Some(entity)
        } else {
            children.first().copied()
        };
        if let Some(mut transform) = target.and_then(|e| scales.get_mut(e).ok()) {
            transform.scale = Vec2::splat(scale);
        }
        if !inner && let Some(mut ground) = ground {
            ground.0 = tokens::SCRIM.with_alpha(tokens::SCRIM.alpha() * shown);
        }
        if pop.leaving && pop.t <= 0.0 {
            commands.entity(entity).try_despawn();
        }
    }
    if sheet.copied > 0.0 {
        sheet.copied = (sheet.copied - motion::sanitize_dt(delta)).max(0.0);
        // In fast, out slowly over its last second.
        let alpha = (sheet.copied / 1.0).min(1.0);
        for children in &copied {
            for child in children.iter() {
                if let Ok(mut ink) = inks.get_mut(child) {
                    ink.0 = tokens::ACCENT.with_alpha(alpha);
                }
            }
        }
    }
}
