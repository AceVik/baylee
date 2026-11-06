//! The order of a player's own triggers (CR 603.3b), between the question and
//! the account: the suggestion is laid out as the question arrives, and what
//! the answer taught goes to the player's preferences. The sheet itself is
//! the zone dialog's (`hud::tray::triggers`), and everything decided here is
//! `baylee_client_core::triggerorder`'s.

use crate::Duel;
use baylee_client_core::arrange::Row;
use baylee_client_core::triggerorder::sheet::{self, Sheet};
use baylee_core::ids::ObjectId;
use bevy::prelude::*;

/// Lays the suggestion out for a new question, sends it when the player
/// allowed that, and files what a sent answer taught.
///
/// Reads first and writes only on a change: a mutable borrow of the `Duel`
/// that changes nothing still reruns every `duel.is_changed()` guard.
pub(crate) fn keep_up(mut duel: ResMut<Duel>, mut prefs: ResMut<crate::prefs::Prefs>) {
    if duel.trigger_answer.is_some() {
        let chosen = duel.trigger_answer.take();
        let sheet = duel.trigger_sheet.take();
        if let (Some(chosen), Some(sheet)) = (chosen, sheet) {
            let mut memory = prefs.all().trigger_order.clone();
            if sheet.teach(&chosen, &mut memory) {
                prefs.edit().trigger_order = memory;
            }
        }
    }
    let asked = duel
        .interaction
        .as_ref()
        .is_some_and(|i| i.is_mine() && i.orders_triggers());
    if !asked {
        if duel.trigger_sheet.is_some() {
            duel.trigger_sheet = None;
        }
        return;
    }
    let (Some(view), Some(statics)) = (duel.view.as_ref(), duel.statics.as_ref()) else {
        return;
    };
    if duel
        .trigger_sheet
        .as_ref()
        .is_some_and(|sheet| sheet.answers(view))
    {
        return;
    }
    let Some(sheet) = Sheet::of(view, statics, &prefs.all().trigger_order) else {
        return;
    };
    let unasked = sheet.goes_unasked();
    if let Some(interaction) = duel.interaction.as_mut() {
        interaction.start_arrangement(sheet.suggested());
    }
    duel.trigger_sheet = Some(sheet);
    if unasked
        && let Some(action) = duel
            .interaction
            .as_ref()
            .and_then(baylee_client_core::Interaction::confirm)
    {
        duel.submit(action);
    }
}

/// The sheet's rows as they stand.
fn runs(duel: &Duel) -> Option<Vec<Vec<ObjectId>>> {
    let pile = duel
        .interaction
        .as_ref()?
        .arrangement()?
        .cards(Row::Pile(0));
    let rows = &duel.view.as_ref()?.trigger_order;
    Some(duel.trigger_sheet.as_ref()?.runs(pile, rows))
}

/// Moves `card`'s row one row earlier or later.
pub(crate) fn step(duel: &mut Duel, card: ObjectId, later: bool) -> bool {
    let order = runs(duel).and_then(|runs| sheet::stepped(&runs, card, later));
    match (order, duel.interaction.as_mut()) {
        (Some(order), Some(interaction)) => interaction.rearrange(&order),
        _ => false,
    }
}

/// Stands `card`'s row where `onto`'s row stands.
pub(crate) fn drop_on(duel: &mut Duel, card: ObjectId, onto: ObjectId) -> bool {
    let order = runs(duel).and_then(|runs| sheet::dropped(&runs, card, onto));
    match (order, duel.interaction.as_mut()) {
        (Some(order), Some(interaction)) => {
            interaction.focus_arrangement_on(card);
            interaction.rearrange(&order)
        }
        _ => false,
    }
}

/// Walks the keyboard one row along the sheet.
pub(crate) fn walk(duel: &mut Duel, later: bool) -> bool {
    let at = duel
        .interaction
        .as_ref()
        .and_then(|i| i.arrangement()?.focused());
    let next = at.and_then(|at| sheet::neighbour(&runs(duel)?, at, later));
    match (next, duel.interaction.as_mut()) {
        (Some(next), Some(interaction)) => {
            interaction.focus_arrangement_on(next);
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::Interaction;
    use baylee_client_core::prefs::Preferences;
    use baylee_client_core::test_support::ViewBuilder;
    use baylee_client_core::triggerorder::{Reason, TriggerKey};
    use baylee_core::ids::{AbilityRef, PlayerId};
    use baylee_engine::choice::{ArrangePile, ArrangePlace, ArrangePrompt, Pending, PlayerAction};
    use baylee_view::{GameStatic, OrderedTrigger, SeatIdentity, StackItem};
    use bevy::ecs::system::RunSystemOnce as _;

    const ME: PlayerId = PlayerId::new(0);
    const THEM: PlayerId = PlayerId::new(1);

    fn card(name: &str) -> AbilityRef {
        let card = baylee_cards::all()
            .find(|card| card.faces[0].name == name)
            .expect("in the pool");
        AbilityRef::new(card.index, 0)
    }

    fn row(place: u32, name: &str, about: PlayerId) -> OrderedTrigger {
        OrderedTrigger {
            handle: ObjectId::queued_trigger(place),
            item: StackItem::Ability {
                source: ObjectId::new(place + 1, 0),
                ability: Some(card(name)),
                text: None,
                rules: None,
                token: None,
            },
            name: name.into(),
            twin: place,
            about_object: None,
            about_name: None,
            about_player: Some(about),
            amount: None,
            asks_targets: false,
            modal: false,
        }
    }

    /// The question, asked as the view and the pending bring it.
    fn asked(rows: Vec<OrderedTrigger>) -> Duel {
        let mut view = ViewBuilder::new(2).build();
        let cards: Vec<ObjectId> = rows.iter().map(|row| row.handle).collect();
        view.trigger_order = rows;
        let count = u32::try_from(cards.len()).unwrap();
        let pending = Pending::Arrange {
            player: ME,
            cards,
            piles: vec![ArrangePile::all_of(ArrangePlace::Stack, count)],
            prompt: ArrangePrompt::TriggerOrder,
        };
        Duel {
            view: Some(view),
            interaction: Some(Interaction::new(pending, ME)),
            statics: Some(GameStatic {
                decision_secs: None,
                reconnect_secs: None,
                view_version: baylee_view::VIEW_VERSION,
                game_id: "g".into(),
                your_seat: ME,
                seats: [ME, THEM]
                    .map(|player| SeatIdentity {
                        player,
                        display_name: "A player".into(),
                        is_ai: false,
                        away: false,
                        team: None,
                    })
                    .to_vec(),
                prints: vec![],
            }),
            ..Duel::default()
        }
    }

    fn world(duel: Duel, preferences: Preferences) -> World {
        let mut prefs = crate::prefs::Prefs::default();
        *prefs.edit() = preferences;
        let mut world = World::new();
        world.insert_resource(duel);
        world.insert_resource(prefs);
        world
    }

    fn pile(world: &World) -> Vec<ObjectId> {
        world
            .resource::<Duel>()
            .interaction
            .as_ref()
            .and_then(Interaction::arrangement)
            .expect("the question stands")
            .cards(Row::Pile(0))
            .to_vec()
    }

    fn egg_about(place: u32, who: PlayerId) -> OrderedTrigger {
        row(place, "Dingus Egg", who)
    }

    /// The offer hits the player first; the sheet opens with the opponent
    /// hit first, the row a player sees before touching anything.
    #[test]
    fn the_suggestion_is_what_the_question_opens_on() {
        let mut world = world(
            asked(vec![egg_about(0, ME), egg_about(1, THEM)]),
            Preferences::default(),
        );
        world.run_system_once(keep_up).unwrap();
        assert_eq!(
            pile(&world),
            vec![ObjectId::queued_trigger(1), ObjectId::queued_trigger(0)]
        );
        let duel = world.resource::<Duel>();
        let sheet = duel.trigger_sheet.as_ref().expect("the sheet");
        assert_eq!(sheet.reason(), Reason::OpponentFirst);
    }

    /// A second frame leaves what the player is building alone.
    #[test]
    fn the_sheet_is_laid_out_once_and_keeps_what_the_player_moved() {
        let mut world = world(
            asked(vec![egg_about(0, ME), egg_about(1, THEM)]),
            Preferences::default(),
        );
        world.run_system_once(keep_up).unwrap();
        let mut duel = world.resource_mut::<Duel>();
        assert!(step(&mut duel, ObjectId::queued_trigger(1), true));
        world.run_system_once(keep_up).unwrap();
        assert_eq!(
            pile(&world),
            vec![ObjectId::queued_trigger(0), ObjectId::queued_trigger(1)]
        );
    }

    fn answer(world: &mut World) {
        let mut duel = world.resource_mut::<Duel>();
        let action = duel
            .interaction
            .as_ref()
            .and_then(Interaction::confirm)
            .expect("a whole answer");
        assert!(matches!(action, PlayerAction::Arrange { .. }));
        duel.submit(action);
    }

    fn memory(world: &World) -> baylee_client_core::triggerorder::memory::TriggerMemory {
        world
            .resource::<crate::prefs::Prefs>()
            .all()
            .trigger_order
            .clone()
    }

    /// Two different cards, the order turned around: it is learned, and the
    /// next question opens on it, marked.
    #[test]
    fn what_the_player_turned_around_is_learned_and_opens_the_next_question() {
        let rows = || vec![row(0, "Copper Tablet", ME), row(1, "Phyrexian Arena", ME)];
        let mut world = world(asked(rows()), Preferences::default());
        world.run_system_once(keep_up).unwrap();
        let first = pile(&world);
        let mut duel = world.resource_mut::<Duel>();
        assert!(step(&mut duel, first[1], false));
        answer(&mut world);
        world.run_system_once(keep_up).unwrap();
        let learned = memory(&world);
        assert_eq!(learned.len(), 1);
        let key = |name| TriggerKey::Card(card(name));
        assert_eq!(
            learned.knows(key("Phyrexian Arena"), key("Copper Tablet")),
            Some(first[1] == ObjectId::queued_trigger(1))
        );

        let preferences = Preferences {
            trigger_order: learned,
            ..Preferences::default()
        };
        let mut world = self::world(asked(rows()), preferences);
        world.run_system_once(keep_up).unwrap();
        let duel = world.resource::<Duel>();
        let sheet = duel.trigger_sheet.as_ref().expect("the sheet");
        assert_eq!(sheet.reason(), Reason::Remembered);
        assert!(sheet.is_remembered(ObjectId::queued_trigger(0)));
    }

    /// A plain confirmation writes nothing: the account is not touched by a
    /// player who agreed.
    #[test]
    fn a_plain_confirmation_touches_no_preferences() {
        let mut world = world(
            asked(vec![
                row(0, "Copper Tablet", ME),
                row(1, "Phyrexian Arena", ME),
            ]),
            Preferences::default(),
        );
        world.run_system_once(keep_up).unwrap();
        answer(&mut world);
        world.run_system_once(keep_up).unwrap();
        assert!(memory(&world).is_empty());
        assert!(world.resource::<Duel>().trigger_answer.is_none());
    }

    /// Off by default, and with it on only a question fully known goes.
    #[test]
    fn a_known_question_goes_unasked_only_when_the_player_allowed_it() {
        let rows = || vec![row(0, "Copper Tablet", ME), row(1, "Phyrexian Arena", ME)];
        let key = |name| Some(TriggerKey::Card(card(name)));
        let mut preferences = Preferences::default();
        preferences.trigger_order.learn(
            &[key("Copper Tablet"), key("Phyrexian Arena")],
            &[key("Phyrexian Arena"), key("Copper Tablet")],
        );
        let mut world_off = world(asked(rows()), preferences.clone());
        world_off.run_system_once(keep_up).unwrap();
        assert!(
            world_off.resource::<Duel>().trigger_answer.is_none(),
            "asked as ever"
        );

        preferences.trigger_order.accept_known = true;
        let mut world_on = world(asked(rows()), preferences);
        world_on.run_system_once(keep_up).unwrap();
        assert!(
            world_on.resource::<Duel>().trigger_answer.is_some(),
            "sent by itself"
        );
    }

    /// A question that ends unanswered by this client (the clock's) leaves
    /// no sheet behind it to teach from.
    #[test]
    fn a_sheet_goes_with_its_question() {
        let mut world = world(
            asked(vec![egg_about(0, ME), egg_about(1, THEM)]),
            Preferences::default(),
        );
        world.run_system_once(keep_up).unwrap();
        world.resource_mut::<Duel>().interaction = None;
        world.run_system_once(keep_up).unwrap();
        assert!(world.resource::<Duel>().trigger_sheet.is_none());
    }
}
