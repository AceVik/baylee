//! A retained drawer over the table. It reads the public view; it never edits rules.
use super::{MenuAction, MenuButton, UiFonts};
use crate::{
    Duel, DuelPhase, DuelSet,
    settings::ClientSettings,
    shellkit::{
        self,
        controls::{Kit, Live, Weight},
        tokens,
    },
    towerui,
    ui_button::UiButton,
};
use baylee_client_core::{Lang, Phrase, echoes::domain::v2::shop::Goods};
use baylee_core::ids::ObjectId;
use bevy::prelude::*;

mod draw;
pub(crate) mod keys;
#[cfg(test)]
mod tests;

#[derive(Component)]
struct Root;

#[derive(Component)]
struct Panel;

#[derive(Clone, PartialEq, Eq)]
struct VisibleEntry {
    id: ObjectId,
    seat: u8,
    goods: Goods,
    charges: u16,
    usable: bool,
}

#[derive(Resource, Default)]
struct Drawn {
    game: Option<String>,
    items: Vec<VisibleEntry>,
    open: bool,
    ability: Option<ObjectId>,
    over: bool,
    watching: bool,
    lang: Option<Lang>,
    canvas: IVec2,
    scale_bits: u32,
    text: Option<shellkit::TextSize>,
}

pub(crate) fn install(app: &mut App) {
    crate::itemart::install(app);
    crate::towerui::install(app);
    shellkit::install(app);
    app.init_resource::<Drawn>()
        .add_systems(
            Update,
            sync.in_set(DuelSet::Present)
                .run_if(not(in_state(DuelPhase::Closed))),
        )
        .add_systems(OnEnter(DuelPhase::Closed), cleanup)
        .add_systems(
            Update,
            (
                keys::activate,
                (super::ledge::log::items::hover_log_items, ApplyDeferred)
                    .chain()
                    .before(shellkit::controls::show_tooltips),
            )
                .run_if(not(in_state(DuelPhase::Closed))),
        );
}

fn cleanup(mut commands: Commands, roots: Query<Entity, With<Root>>, mut drawn: ResMut<Drawn>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
    *drawn = Drawn::default();
}

#[allow(clippy::too_many_arguments)]
fn sync(
    mut commands: Commands,
    duel: Res<Duel>,
    fonts: Res<UiFonts>,
    settings: Res<ClientSettings>,
    (windows, scale): (Query<&Window>, Option<Res<UiScale>>),
    roots: Query<Entity, With<Root>>,
    panels: Query<&ScrollPosition, With<Panel>>,
    mut sounds: Option<MessageWriter<crate::sound::tower::TowerSound>>,
    mut drawn: ResMut<Drawn>,
) {
    let canvas =
        super::scale::window_space(&windows, scale.as_deref()).unwrap_or(Vec2::new(1280.0, 720.0));
    let (win_w, win_h) = windows
        .single()
        .map_or((1280.0, 720.0), |w| (w.width(), w.height()));
    let hud_scale = scale.as_ref().map_or(1.0, |s| s.0);
    let size = canvas.as_ivec2();
    let game = duel.statics.as_ref().map(|game| game.game_id.as_str());
    let lang = Lang::of(&settings.lang);
    let over = duel.ending().is_some();
    let items = snapshot_items(&duel);

    if drawn.game.as_deref() == game {
        if !drawn.open && duel.tower_inventory {
            if let Some(sounds) = sounds.as_mut() {
                sounds.write(crate::sound::tower::TowerSound::DrawerOpen);
            }
        } else if drawn.ability != duel.ability_menu
            && duel.ability_menu.is_some_and(|id| can_use(&duel, id))
            && let Some(sounds) = sounds.as_mut()
        {
            sounds.write(crate::sound::tower::TowerSound::ItemUse);
        }
    }

    if drawn.game.as_deref() == game
        && drawn.items == items
        && drawn.open == duel.tower_inventory
        && drawn.ability == duel.ability_menu
        && drawn.lang == Some(lang)
        && drawn.canvas == size
        && drawn.scale_bits == hud_scale.to_bits()
        && drawn.over == over
        && drawn.watching == duel.watching
        && drawn.text == Some(settings.text_size)
    {
        return;
    }

    let kept_scroll = if drawn.game.as_deref() == game && drawn.open && duel.tower_inventory {
        panels.iter().next().map_or(0.0, |pos| pos.0.y)
    } else {
        0.0
    };

    *drawn = Drawn {
        game: game.map(str::to_owned),
        items,
        open: duel.tower_inventory,
        ability: duel.ability_menu,
        over,
        watching: duel.watching,
        lang: Some(lang),
        canvas: size,
        scale_bits: hud_scale.to_bits(),
        text: Some(settings.text_size),
    };
    for root in &roots {
        commands.entity(root).despawn();
    }
    if over || drawn.items.is_empty() {
        return;
    }
    let kit = Kit {
        fonts: &fonts,
        german: lang == Lang::De,
        m: shellkit::ShellMetrics::under(
            shellkit::Viewport::desktop(win_w, win_h),
            settings.text_size,
            hud_scale,
        ),
    };
    draw::draw(&mut commands, kit, &duel, canvas, lang, kept_scroll);
}

fn snapshot_items(duel: &Duel) -> Vec<VisibleEntry> {
    let Some(view) = &duel.view else {
        return Vec::new();
    };
    let mut entries = Vec::new();
    for (seat, objects) in view.command.iter().enumerate() {
        let seat_u8 = u8::try_from(seat).unwrap_or(u8::MAX);
        for object in objects {
            if let Some(goods) = visible(object, view.seat, duel.watching) {
                entries.push(VisibleEntry {
                    id: object.id,
                    seat: seat_u8,
                    goods,
                    charges: object.counter_count(baylee_view::CounterKind::Charge),
                    usable: can_use(duel, object.id),
                });
            }
        }
    }
    entries
}

fn visible(
    object: &baylee_view::PublicObject,
    reader: baylee_core::ids::PlayerId,
    watching: bool,
) -> Option<Goods> {
    let goods = crate::itemart::identity::run_source(object)?;
    (!matches!(goods, Goods::Potion(_)) || (!watching && object.owner == reader)).then_some(goods)
}

/// Revalidate against the current view. Opening the standard chooser spends nothing.
pub(crate) fn use_item(duel: &mut Duel, object: ObjectId) {
    if can_use(duel, object) {
        duel.tower_inventory = false;
        duel.ability_menu = Some(object);
        duel.ability_pick = 0;
        duel.ability_page = 0;
        duel.ability_tap = None;
    }
}

pub(crate) fn click(duel: &mut Duel, action: MenuAction) {
    match action {
        MenuAction::ToggleTowerItems => {
            if !available(duel) {
                return;
            }
            duel.tower_inventory = !duel.tower_inventory;
            duel.game_menu = false;
        }
        MenuAction::UseTowerItem(object) => use_item(duel, object),
        _ => {}
    }
}

pub(crate) fn available(duel: &Duel) -> bool {
    duel.ending().is_none()
        && duel.view.as_ref().is_some_and(|view| {
            view.command
                .iter()
                .flatten()
                .any(|object| visible(object, view.seat, duel.watching).is_some())
        })
}

pub(crate) fn can_use(duel: &Duel, object: ObjectId) -> bool {
    !duel.watching
        && duel.ending().is_none()
        && duel.view.as_ref().is_some_and(|view| {
            view.command
                .get(usize::from(view.seat.get()))
                .is_some_and(|objects| {
                    objects.iter().any(|entry| {
                        entry.id == object
                            && entry.owner == view.seat
                            && crate::itemart::identity::auxiliary(entry)
                    })
                })
        })
        && super::ability_options(duel, Lang::En, object).is_some()
}

/// Spawns a rich Tower Item Detail Overlay (`FloatingTooltip`) with separated
/// Item Name and Rarity subline, and attaches it to `target` via
/// `(Tooltip(tip), FocusTooltip)`.
pub(in crate::hud) fn attach_item_tooltip(
    commands: &mut Commands,
    target: Entity,
    goods: Goods,
    fonts: &UiFonts,
    lang: Lang,
) -> Entity {
    towerui::attach_item_tooltip(
        commands,
        target,
        towerui::kit_for_hud(fonts, lang),
        goods,
        lang,
    )
}
