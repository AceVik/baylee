use super::{Action, Book, ORDER, Origin, Page, worker};
use crate::{
    event_cosmetics::CosmeticPreview,
    hud::UiFonts,
    lobby::LobbyState,
    shellkit::{
        InputClass, ShellMetrics,
        controls::{self, Kit, Live, Weight},
        focus::Stop,
        surfaces::{self, SheetHeader, SheetWidth},
        tokens,
    },
};
use baylee_account::advent::{Id, Kind, Reward};
use baylee_client_core::i18n::{Lang, Phrase};
use bevy::prelude::*;

const PER_PAGE: usize = 12;
#[derive(Component)]
pub(super) struct Preview(Id);
#[derive(Default)]
pub(super) struct Paint {
    shell: Option<(Entity, Entity, Entity, ShellMetrics, Lang)>,
    inventory: Option<(usize, bool, bool, bool)>,
}

fn column(c: &mut Commands, kit: Kit) -> Entity {
    c.spawn(Node {
        width: percent(100),
        flex_direction: FlexDirection::Column,
        row_gap: kit.m.px(8.0),
        flex_shrink: 0.,
        ..default()
    })
    .id()
}
fn line(c: &mut Commands, parent: Entity, kit: Kit, text: &str) {
    let row = c
        .spawn((
            Text::new(text),
            crate::hud::tf(kit.fonts, kit.m.text),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    c.entity(parent).add_child(row);
}
fn button(
    c: &mut Commands,
    parent: Entity,
    kit: Kit,
    label: &str,
    action: Action,
    stop: Stop,
    chosen: bool,
) {
    let weight = if chosen {
        Weight::Primary
    } else {
        Weight::Secondary
    };
    let row = controls::button(
        c,
        kit,
        label,
        weight,
        Live::Yes,
        None,
        (action, stop, crate::shellkit::focus::Current(chosen)),
    );
    c.entity(parent).add_child(row);
}
fn stop(key: &'static str) -> Stop {
    Stop::new(ORDER.name, key)
}
fn choice(index: usize) -> Stop {
    Stop::item(ORDER.name, "item", u8::try_from(index).unwrap_or_default())
}
fn name(r: &Reward, lang: Lang) -> &str {
    if lang == Lang::De {
        &r.title.de
    } else {
        &r.title.en
    }
}
fn title(kind: Kind) -> Phrase {
    match kind {
        Kind::Sleeve => Phrase::DeckAppearanceSleeve,
        Kind::Playmat => Phrase::DeckAppearanceMat,
    }
}
fn paging(c: &mut Commands, body: Entity, kit: Kit, page: usize, length: usize) {
    if page > 0 {
        button(
            c,
            body,
            kit,
            Phrase::DeckAppearancePrevious.text(kit.lang()),
            Action::Page(page - 1),
            stop("previous"),
            false,
        );
    }
    if page + 1 < length.div_ceil(PER_PAGE) {
        button(
            c,
            body,
            kit,
            Phrase::DeckAppearanceNext.text(kit.lang()),
            Action::Page(page + 1),
            stop("next"),
            false,
        );
    }
}

fn preview(c: &mut Commands, body: Entity, kit: Kit, reward: &Reward, book: &Book) {
    let (w, h) = match reward.kind {
        Kind::Sleeve => (100., 150.),
        Kind::Playmat => (200., 112.),
    };
    let root = c
        .spawn((
            Preview(reward.id),
            Node {
                width: kit.m.px(w),
                height: kit.m.px(h),
                align_self: AlignSelf::Center,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_shrink: 0.,
                ..default()
            },
            BackgroundColor(tokens::CONTROL),
            Pickable::IGNORE,
        ))
        .id();
    if let Some(image) = book.image(reward.id) {
        c.entity(root).insert(CosmeticPreview::new(
            image.clone(),
            reward.kind,
            reward.asset.tier,
        ));
    } else if book.image_error(reward.id).is_some() {
        line(
            c,
            root,
            kit,
            Phrase::DeckAppearanceUnavailable.text(kit.lang()),
        );
        button(
            c,
            body,
            kit,
            Phrase::DeckAppearanceRetry.text(kit.lang()),
            Action::RetryImage,
            stop("retry"),
            false,
        );
    } else {
        line(c, root, kit, Phrase::ChallengeLoading.text(kit.lang()));
    }
    c.entity(body).add_child(root);
    line(c, body, kit, name(reward, kit.lang()));
}

fn deck_list(c: &mut Commands, body: Entity, kit: Kit, p: &Page) {
    let prompt = if p.decks.is_empty() {
        Phrase::DeckAppearanceNoDecks
    } else {
        Phrase::DeckAppearanceChooseDeck
    };
    line(c, body, kit, prompt.text(kit.lang()));
    for (idx, (id, name)) in p
        .decks
        .iter()
        .skip(p.list_index.saturating_mul(PER_PAGE))
        .take(PER_PAGE)
        .enumerate()
    {
        button(c, body, kit, name, Action::Deck(*id), choice(idx), false);
    }
    paging(c, body, kit, p.list_index, p.decks.len());
}

fn contents(c: &mut Commands, body: Entity, foot: Entity, kit: Kit, p: &Page, b: &Book) {
    let lang = kit.lang();
    if p.problem.is_some() || b.inventory_error().is_some() {
        let why = if p.problem == Some(baylee_account::Error::Conflict) {
            Phrase::DeckAppearanceChanged
        } else {
            Phrase::DeckAppearanceFailed
        };
        line(c, body, kit, why.text(lang));
        button(
            c,
            foot,
            kit,
            Phrase::DeckAppearanceRetry.text(lang),
            Action::Retry,
            stop("retry"),
            false,
        );
        return;
    }
    if p.pending.is_some() || !b.inventory_loaded() {
        line(c, body, kit, Phrase::ChallengeLoading.text(lang));
        return;
    }
    let Some(d) = &p.deck else {
        deck_list(c, body, kit, p);
        return;
    };
    let kind = p.kind.unwrap_or(Kind::Sleeve);
    line(c, body, kit, &d.name);
    if matches!(p.origin, Some(Origin::Editor(_))) {
        let tabs = c
            .spawn(Node {
                width: percent(100),
                justify_content: JustifyContent::Center,
                flex_wrap: FlexWrap::Wrap,
                column_gap: kit.m.px(8.),
                row_gap: kit.m.px(4.),
                ..default()
            })
            .id();
        c.entity(body).add_child(tabs);
        for (slot, key) in [(Kind::Sleeve, "sleeve"), (Kind::Playmat, "mat")] {
            button(
                c,
                tabs,
                kit,
                title(slot).text(lang),
                Action::Slot(slot),
                stop(key),
                slot == kind,
            );
        }
    }
    if let Some(item) = p
        .selected
        .and_then(|id| b.inventory().iter().find(|r| r.id == id))
    {
        preview(c, body, kit, item, b);
    } else if p.selected.is_some() {
        line(c, body, kit, Phrase::DeckAppearanceUnavailable.text(lang));
    }
    if matches!(p.origin, Some(Origin::Editor(_))) {
        designs(c, body, kit, p, b, kind);
    }
    if worker::slot(d.look, kind) == p.selected {
        line(
            c,
            foot,
            kit,
            if p.saved {
                Phrase::DeckAppearanceSaved
            } else {
                Phrase::DeckAppearanceCurrent
            }
            .text(lang),
        );
    } else {
        button(
            c,
            foot,
            kit,
            Phrase::DeckAppearanceApply.text(lang),
            Action::Apply,
            stop("apply"),
            true,
        );
    }
}

fn designs(c: &mut Commands, body: Entity, kit: Kit, p: &Page, b: &Book, kind: Kind) {
    let lang = kit.lang();
    button(
        c,
        body,
        kit,
        Phrase::DeckAppearanceDefault.text(lang),
        Action::Select(None),
        stop("default"),
        p.selected.is_none(),
    );
    let items: Vec<_> = b.inventory().iter().filter(|r| r.kind == kind).collect();
    if items.is_empty() {
        line(c, body, kit, Phrase::DeckAppearanceNoItems.text(lang));
    }
    for (idx, item) in items
        .iter()
        .skip(p.list_index.saturating_mul(PER_PAGE))
        .take(PER_PAGE)
        .enumerate()
    {
        button(
            c,
            body,
            kit,
            name(item, lang),
            Action::Select(Some(item.id)),
            choice(idx),
            Some(item.id) == p.selected,
        );
    }
    paging(c, body, kit, p.list_index, items.len());
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render(
    mut commands: Commands,
    page: Res<Page>,
    book: Res<Book>,
    lobby: Res<LobbyState>,
    fonts: Res<UiFonts>,
    windows: Query<&Window>,
    input: Option<Res<InputClass>>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    mut paint: Local<Paint>,
    previews: Query<(Entity, &Preview), Without<CosmeticPreview>>,
) {
    if page.origin.is_none()
        || page
            .owner
            .as_ref()
            .is_none_or(|owner| !book.belongs_to(owner))
    {
        if let Some((root, ..)) = paint.shell.take() {
            commands.entity(root).try_despawn();
            paint.inventory = None;
        }
        return;
    }
    let lang = lobby.lobby.lang();
    let (width, height) = windows
        .iter()
        .next()
        .map_or((1024., 720.), |window| (window.width(), window.height()));
    let metrics = crate::lobby::header::kit_metrics(
        width,
        height,
        settings.as_deref().map_or_default(|s| s.text_size),
        input.as_deref().copied().unwrap_or_default(),
    );
    let kit = Kit {
        m: metrics,
        fonts: &fonts,
        german: lang == Lang::De,
    };
    let same_shell = paint
        .shell
        .is_some_and(|(_, _, _, old, old_lang)| old == metrics && old_lang == lang);
    let inventory = (
        book.inventory().len(),
        book.inventory_loaded(),
        book.inventory_error().is_some(),
        page.selected
            .is_some_and(|id| book.image_error(id).is_some()),
    );
    if same_shell && !page.is_changed() && paint.inventory == Some(inventory) {
        if book.is_changed() {
            for (entity, preview) in &previews {
                if let Some(image) = book.image(preview.0)
                    && let Some(reward) = book
                        .inventory()
                        .iter()
                        .find(|reward| reward.id == preview.0)
                {
                    commands
                        .entity(entity)
                        .despawn_children()
                        .insert(CosmeticPreview::new(
                            image.clone(),
                            reward.kind,
                            reward.asset.tier,
                        ));
                }
            }
        }
        return;
    }
    let (body, foot) = if same_shell && let Some((_, body, foot, ..)) = paint.shell {
        commands.entity(body).despawn_children();
        commands.entity(foot).despawn_children();
        (body, foot)
    } else {
        if let Some((old, ..)) = paint.shell.take() {
            commands.entity(old).try_despawn();
        }
        let body = column(&mut commands, kit);
        let foot = column(&mut commands, kit);
        let header = SheetHeader::new(Phrase::DeckAppearance.text(lang))
            .close((Action::Close, stop("close")));
        let sheet = surfaces::sheet_box_fixed(
            &mut commands,
            kit,
            SheetWidth::Medium,
            header,
            &[body],
            &[foot],
        );
        let root = surfaces::scrim(
            &mut commands,
            kit,
            crate::social::window::G_PANEL + 180,
            (),
            sheet,
        );
        paint.shell = Some((root, body, foot, metrics, lang));
        (body, foot)
    };
    contents(&mut commands, body, foot, kit, &page, &book);
    paint.inventory = Some(inventory);
}
