//! Public run-item identities in the log use the same portrait and details as
//! the shop. Never infer an item from a displayed name or look at current zones.
use super::{Commands, Entity, Lang, LogLine, LogLink, UiFonts};
use baylee_client_core::echoes::domain::v2::shop::Goods;

fn goods(link: LogLink) -> Option<Goods> {
    link.card.is_none().then_some(())?;
    crate::itemart::identity::run_token(link.token?)
}

pub(super) fn is_item(link: LogLink) -> bool {
    goods(link).is_some()
}

pub(super) fn portraits(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    line: &LogLine,
    row: Entity,
    text_size: f32,
) {
    // A log entry can mention the same source repeatedly. Four distinct icons
    // are enough for its margin; every name remains in the complete sentence.
    let mut shown = [None; 4];
    let mut count = 0;
    for item in line.names.iter().filter_map(LogLink::of).filter_map(goods) {
        if shown[..count].contains(&Some(item)) {
            continue;
        }
        shown[count] = Some(item);
        count += 1;
        let icon = crate::towerui::inspectable(commands, item, fonts, lang, text_size * 1.6);
        commands.entity(row).add_child(icon);
        if count == shown.len() {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    #[test]
    fn log_items_use_public_identity_and_shared_details_instead_of_fake_card_links() {
        let mut line = super::super::tests::line("Potion Potion", &[(0, 6), (7, 13)], 1);
        let token = baylee_cards::tokens::token_id(&baylee_cards::tokens::TOWER_HEALING_POTION);
        for name in &mut line.names {
            name.token = Some(token);
        }
        let mut world = World::new();
        let fonts = UiFonts::default();
        let row = super::super::spawn_line(
            &mut world.commands(),
            &fonts,
            Lang::En,
            &line,
            &super::super::PANEL_INKS,
            None,
        );
        world.flush();
        assert!(world.get_entity(row).is_ok());
        assert_eq!(world.query::<&LogLink>().iter(&world).count(), 0);
        // One margin icon and its shared tooltip's larger portrait, not two
        // duplicate margin icons for two mentions of the same item.
        assert_eq!(
            world
                .query::<&crate::itemart::Portrait>()
                .iter(&world)
                .count(),
            2
        );
        assert!(!is_item(LogLink {
            card: None,
            token: None
        }));
        assert!(!is_item(LogLink {
            card: None,
            token: Some(u16::MAX)
        }));
        assert!(!is_item(LogLink {
            card: Some(super::super::tests::bolt_identity()),
            token: Some(token)
        }));
    }
}
