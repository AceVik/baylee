//! Shared gateway, bell and account controls for every navigation bar.

#[allow(clippy::wildcard_imports)] // the header module's shared vocabulary
use super::*;

/// The actions at the right end of a navigation bar.
pub struct StatusActions<G, B, A, R> {
    /// Opens the gateway menu.
    pub gateway: G,
    /// Opens notifications.
    pub bell: B,
    /// Opens the account menu.
    pub account: A,
    /// Returns to the seated table on a phone.
    pub back: R,
}

/// Adds shared status controls. An editor uses compact status to leave room
/// for its contextual actions; the popovers still show the full details.
#[allow(clippy::too_many_lines)] // one status group, in display order
pub fn status<G: Bundle, B: Bundle, A: Bundle, R: Bundle>(
    commands: &mut Commands,
    kit: Kit,
    bar: Entity,
    look: &HeaderLook,
    actions: StatusActions<G, B, A, R>,
    compact: bool,
) {
    let phone = kit.m.frame == Frame::Phone;
    let wide = matches!(kit.m.frame, Frame::Wide | Frame::Vast);
    let roomy = !compact && wide;
    if let Some(back) = look.return_pill.filter(|_| phone) {
        let pill = controls::button(
            commands,
            kit,
            back,
            Weight::Gold,
            Live::Yes,
            None,
            actions.back,
        );
        commands.entity(bar).add_child(pill);
    } else if roomy {
        let pill = gateway_pill(commands, kit, look, actions.gateway);
        commands.entity(pill).insert(HeaderTrigger::Gateway);
        commands.entity(bar).add_child(pill);
    } else {
        let dot = commands
            .spawn((
                Node {
                    width: kit.m.px(10.0),
                    height: kit.m.px(10.0),
                    border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                    ..default()
                },
                BackgroundColor(look.reach.colour()),
                UiTransform::default(),
                crate::ambience::Feel::new(look.reach.colour()),
            ))
            .id();
        let pill = hit(commands, kit, dot, actions.gateway);
        commands.entity(pill).insert(HeaderTrigger::Gateway);
        commands.entity(bar).add_child(pill);
    }
    commands.entity(bar).add_children(look.tools);
    let bell = bell(commands, kit, look.unread, actions.bell);
    commands.entity(bell).insert(HeaderTrigger::Bell);
    commands.entity(bar).add_child(bell);
    if let Some(handle) = look.handle {
        let account = if wide {
            controls::pill(commands, kit, None, handle, true, actions.account)
        } else {
            avatar(commands, kit, handle, actions.account)
        };
        commands.entity(account).insert(HeaderTrigger::Account);
        commands.entity(bar).add_child(account);
    }
}
