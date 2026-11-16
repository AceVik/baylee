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

#[derive(Resource, Default)]
struct Drawn {
    game: Option<String>,
    seq: Option<u64>,
    open: bool,
    ability: Option<ObjectId>,
    over: bool,
    watching: bool,
    lang: Option<Lang>,
    canvas: IVec2,
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
            keys::activate.run_if(not(in_state(DuelPhase::Closed))),
        );
}

fn cleanup(mut commands: Commands, roots: Query<Entity, With<Root>>, mut drawn: ResMut<Drawn>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
    *drawn = Drawn::default();
}

fn sync(
    mut commands: Commands,
    duel: Res<Duel>,
    fonts: Res<UiFonts>,
    settings: Res<ClientSettings>,
    (windows, scale): (Query<&Window>, Option<Res<UiScale>>),
    roots: Query<Entity, With<Root>>,
    mut drawn: ResMut<Drawn>,
) {
    let canvas =
        super::scale::window_space(&windows, scale.as_deref()).unwrap_or(Vec2::new(1280.0, 720.0));
    let size = canvas.as_ivec2();
    let game = duel.statics.as_ref().map(|game| game.game_id.as_str());
    let seq = duel.view.as_ref().map(|view| view.seq);
    let lang = Lang::of(&settings.lang);
    let over = duel.ending().is_some();
    if drawn.game.as_deref() == game
        && drawn.seq == seq
        && drawn.open == duel.tower_inventory
        && drawn.ability == duel.ability_menu
        && drawn.lang == Some(lang)
        && drawn.canvas == size
        && drawn.over == over
        && drawn.watching == duel.watching
        && drawn.text == Some(settings.text_size)
    {
        return;
    }
    *drawn = Drawn {
        game: game.map(str::to_owned),
        seq,
        open: duel.tower_inventory,
        ability: duel.ability_menu,
        over,
        watching: duel.watching,
        lang: Some(lang),
        canvas: size,
        text: Some(settings.text_size),
    };
    for root in &roots {
        commands.entity(root).despawn();
    }
    let Some(view) = &duel.view else { return };
    if over
        || !view
            .command
            .iter()
            .flatten()
            .any(|object| visible(object, view.seat, duel.watching).is_some())
    {
        return;
    }
    let kit = Kit {
        fonts: &fonts,
        german: lang == Lang::De,
        m: shellkit::ShellMetrics::of(
            shellkit::Viewport::desktop(canvas.x, canvas.y),
            settings.text_size,
        ),
    };
    draw::draw(&mut commands, kit, &duel, canvas, lang);
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
