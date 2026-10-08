use super::*;
use baylee_core::ids::PlayerId;
use baylee_core::mana::ManaCost;
use baylee_engine::win::{EndReason, GameResult, Victor};

/// Fonts with no asset server behind them: what is under test is which
/// lines the bar builds, and none of that is the GPU's.
fn fonts() -> UiFonts {
    UiFonts {
        text: Handle::default(),
        medium: Handle::default(),
        bold: Handle::default(),
        italic: Handle::default(),
        medium_italic: Handle::default(),
        serif: Handle::default(),
        serif_italic: Handle::default(),
        icons: Handle::default(),
        mana: Handle::default(),
    }
}

/// A view holding one permanent, which is the named card.
///
/// `test_support::printed` keys the identity off a `u16` print number;
/// the registry index is written over it afterwards, because what this
/// asks about is the *card* and a made-up index would answer `false`
/// however the predicate was written.
fn hovering(name: &str) -> (PlayerView, Option<ObjectId>) {
    let index = baylee_cards::decks::by_name(name).expect("in the pool");
    let mut object = baylee_client_core::test_support::printed(1, 0, name, 1);
    object.card.as_mut().expect("printed gives it a card").index = index;
    let id = object.id;
    let view = baylee_client_core::test_support::ViewBuilder::new(2)
        .with_battlefield(0, [object])
        .build();
    (view, Some(id))
}

/// A headless app that really runs [`sync_overlay`].
///
/// Not a source-reading test, because the claim is about what the system
/// *builds* rather than about a component only a renderer creates. The
/// two optional material resources are left out on purpose — that is the
/// branch a machine with no GPU takes, and it is the branch that draws
/// the prose this test reads.
fn bar_of(duel: Duel) -> App {
    // The stack panel asks for pictures, and an `AssetServer::load`
    // spawns on the IO pool and panics without it. Idempotent, so this
    // and `overlay_with` may both ask; here because a duel with a stack
    // reaches that panel from this harness too.
    bevy::tasks::IoTaskPool::get_or_init(Default::default);
    let mut app = App::new();
    app.add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<Image>();
    let textures = {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        CardTextures::new(&mut images, 1 << 20)
    };
    // Motion off, and a clock present at all. The drawer's way out is a
    // movement now, so "shut" means "gone" only once that movement has
    // run — and a bare `App` has no `Time`, so without both of these a
    // dismissed panel would sit at `t = 0` for ever and anything counting
    // panels would be counting one the question had already left. It is
    // the end of the movement and not its absence: see `hud::motion`.
    //
    // The zone dialog joined that on 19.09.2026, when it gained a flight
    // into the tray. Its despawn is `reveal_tray`'s, at the end of the
    // movement, so a harness without both of these would count a sheet
    // for ever after the browser was shut — and with `reduce_motion` the
    // flight is over on the frame it starts, which is what keeps every
    // test written before it meaning what it meant.
    let mut prefs = crate::prefs::Prefs::default();
    prefs.edit().reduce_motion = true;
    app.insert_resource(textures)
        .insert_resource(duel)
        .insert_resource(fonts())
        .insert_resource(crate::settings::ClientSettings::default())
        .insert_resource(prefs)
        .init_resource::<Time>()
        .init_resource::<HudRevision>()
        .init_resource::<crate::cardtext::CardTexts>()
        .init_resource::<crate::face::FaceMode>()
        .init_resource::<crate::sheen::Sheen>()
        .init_resource::<crate::touch::Touched>()
        .init_resource::<ledge::LedgeRevision>()
        .init_resource::<ledge::LedgeLayout>()
        .init_resource::<ledge::drawer::DrawerRevision>()
        .init_resource::<ledge::pool::PoolRevision>()
        .init_resource::<ledge::players::PlayersRevision>()
        .init_resource::<ledge::menu::MenuRevision>()
        .init_resource::<tray::TrayRevision>()
        .init_resource::<tray::TrayReveal>()
        .init_resource::<super::SheetRevision>()
        .insert_resource(super::UiSheets {
            parchment: Handle::default(),
        })
        // All of them, chained, in the order the app runs them: the
        // first spawns the shelf, the three retained attachments and the
        // drawer's node, the second writes the shelf and records where
        // its middle ended up, then the drawer is filled over that middle
        // and opened or shut, the pool's row is reconciled and moved, and
        // the game menu's panel is filled and moved. A harness that ran
        // only the rebuild would be reading a bar with no words on it and
        // calling that an answer.
        //
        // The zone dialog is the last and hangs off the same root on a
        // gate of its own, which is a thing a harness running only
        // `sync_overlay` could no longer see at all.
        .add_systems(
            Update,
            (
                sync_overlay,
                ledge::sync_ledge,
                ledge::drawer::sync_drawer,
                ledge::drawer::zoom_the_drawer,
                ledge::pool::sync_pool,
                ledge::pool::grow_the_pool,
                ledge::players::sync_players,
                ledge::players::glow_the_players,
                ledge::players::show_the_tags,
                ledge::menu::sync_menu,
                ledge::menu::grow_the_menu,
                tray::sync_tray,
                tray::reveal_tray,
                // The parchment leaf, which since AX 6c draws *both*
                // choosers — a permanent's abilities and the ways a card
                // in hand can be cast. A harness that ran the drawer and
                // not this one could watch a row leave the drawer and
                // would have nothing to say about where it went.
                super::sync_ability_sheet,
            )
                .chain(),
        );
    app.update();
    app
}

/// The refusal the engine handed back on the last action anyone took.
const REFUSED: &str = "illegal action for your seat";

fn duel_with(over: bool) -> Duel {
    duel_saying(over, true)
}

/// The same, with the word about the connection left out.
///
/// The shelf shows **one** sentence (AX §6), so a refusal and a lost
/// socket cannot both be read at once — the socket wins, because a
/// question answered into a table that is not there arrives nowhere. On
/// the slip they were two stacked lines and both were drawn.
fn duel_saying(over: bool, unreachable: bool) -> Duel {
    let pending = if over {
        baylee_engine::choice::Pending::GameOver(GameResult {
            winner: Some(Victor::Player(PlayerId::new(0))),
            reason: EndReason::LastPlayerStanding,
        })
    } else {
        baylee_engine::choice::Pending::Priority {
            player: PlayerId::new(0),
            legal: Box::new(baylee_engine::choice::LegalActions::default()),
        }
    };
    let mut duel = Duel {
        interaction: Some(baylee_client_core::Interaction::new(
            pending,
            PlayerId::new(0),
        )),
        last_error: Some(baylee_client_core::i18n::Refusal::Verbatim(
            REFUSED.to_string(),
        )),
        link_note: unreachable.then_some(Phrase::LinkLost),
        ..Duel::default()
    };
    // The bar is not drawn at all without a board to draw it over, which
    // is what an empty tree would otherwise be mistaken for.
    duel.view = Some(baylee_client_core::test_support::ViewBuilder::new(2).build());
    crate::rebuild_board(&mut duel);
    duel
}

/// A priority with something on the stack to let go of, and no hold
/// running.
/// A seat with one spell on the stack, and a table for it to be on.
///
/// The roster is not decoration and was missing: `sync_overlay` draws the
/// stack panel only for a duel whose `statics` have arrived, so this
/// helper promised a stack in its name and drew the shelf's hold button
/// and nothing else. Nothing was asserting vacuously because of it — the
/// two tests on it claim about the shelf — but the next negative claim
/// about the panel would have been, which is the shape that counts a
/// green test that checks nothing.
///
/// `statics` gates the **hand zone** as well, and no other `duel_*`
/// builder here carries one either. Those are sound today for the same
/// reason and for no better one.
fn duel_with_a_stack() -> Duel {
    let mut duel = duel_saying(false, false);
    let view = duel.view.as_mut().expect("the seat has a view");
    view.stack = vec![baylee_client_core::test_support::token(9, 1, "Shock", 0, 0)];
    duel.statics = Some(baylee_client_core::test_support::statics(8));
    crate::rebuild_board(&mut duel);
    duel
}

/// A seat that is not being asked, because it said not to ask.
///
/// The question belongs to the *other* player, which is what makes this
/// seat wait: `Interaction::is_mine` compares the pending's player with
/// the seat, and everything the shelf calls "waiting" comes off that one
/// answer.
fn duel_not_asking(hold: bool, pilot: bool) -> Duel {
    let mut duel = Duel {
        interaction: Some(baylee_client_core::Interaction::new(
            baylee_engine::choice::Pending::Priority {
                player: PlayerId::new(1),
                legal: Box::new(baylee_engine::choice::LegalActions::default()),
            },
            PlayerId::new(0),
        )),
        autopilot: pilot
            .then_some(baylee_client_core::automation::AutoPilot::ToNextTurn { from_turn: 1 }),
        ..Duel::default()
    };
    let mut view = baylee_client_core::test_support::ViewBuilder::new(2).build();
    view.priority_held = hold;
    duel.view = Some(view);
    crate::rebuild_board(&mut duel);
    duel
}

/// A seat being asked to name a colour, which is the shortest question
/// that needs a drawer: the answer is a list, and a list is more than a
/// line.
fn duel_choosing_a_colour() -> Duel {
    use baylee_core::mana::ManaColor;
    let mut duel = Duel {
        interaction: Some(baylee_client_core::Interaction::new(
            baylee_engine::choice::Pending::ChooseColor {
                player: PlayerId::new(0),
                options: vec![ManaColor::White, ManaColor::Blue, ManaColor::Black],
            },
            PlayerId::new(0),
        )),
        ..Duel::default()
    };
    duel.view = Some(baylee_client_core::test_support::ViewBuilder::new(2).build());
    crate::rebuild_board(&mut duel);
    duel
}

mod answer_tests;
mod decision_tests;
mod menu_tests;
mod preview_tests;
mod sheet_tests;
mod shelf_tests;
mod stack_tests;
mod strip_tests;

use answer_tests::*;
use strip_tests::*;
