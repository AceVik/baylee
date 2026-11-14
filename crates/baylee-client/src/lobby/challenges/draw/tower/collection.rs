//! Distinct tower relics and their permanent, progressively ornamented material tiers.
#[allow(clippy::wildcard_imports)]
use super::*;
use tower_api::BadgeKind;

pub(super) fn draw(commands: &mut Commands, body: Entity, kit: Kit, lang: Lang, model: &State) {
    let section = panel(commands, body, kit);
    heading(commands, section, kit, Phrase::TowerRewards.text(lang));
    let gallery = row(commands, section, kit);
    for kind in BadgeKind::ALL {
        let progress = model.tower.badges.iter().find(|badge| badge.kind == kind);
        let cell = column(commands, gallery, kit, 30.0);
        commands
            .entity(cell)
            .entry::<Node>()
            .and_modify(|mut node| node.align_items = AlignItems::Center);
        medal(
            commands,
            cell,
            kit,
            kind,
            progress.and_then(|badge| badge.tier),
            112.0,
        );
        control(
            commands,
            cell,
            kit,
            lang,
            name(kind),
            TowerAction::Badge(kind),
            model,
            Weight::Ghost,
        );
        label(
            commands,
            cell,
            kit,
            super::super::rewards::tier(progress.and_then(|badge| badge.tier)).text(lang),
            if progress.is_some_and(|badge| badge.tier.is_some()) {
                tokens::GOLD
            } else {
                tokens::MUTED
            },
        );
        let count = progress.map_or(0, |badge| badge.count);
        let next = progress.map_or(Some(1), |badge| badge.next);
        let caption = next.map_or_else(
            || Phrase::ChallengeBadgeMax.fill(lang, &[&count.to_string()]),
            |next| {
                Phrase::ChallengeBadgeCompact.fill(lang, &[&count.to_string(), &next.to_string()])
            },
        );
        label(commands, cell, kit, &caption, tokens::MUTED);
        super::super::rewards::meter(commands, cell, kit, count, next);
    }
    if let Some(kind) = model.tower.badge {
        preview(commands, section, kit, lang, model, kind);
    }
}

fn preview(
    commands: &mut Commands,
    section: Entity,
    kit: Kit,
    lang: Lang,
    model: &State,
    kind: BadgeKind,
) {
    let description = match kind {
        BadgeKind::Pathfinder => Phrase::TowerEarnPathfinder,
        BadgeKind::Summit => Phrase::TowerEarnSummit,
        BadgeKind::Unbroken => Phrase::TowerEarnUnbroken,
    };
    label(commands, section, kit, description.text(lang), tokens::INK);
    label(
        commands,
        section,
        kit,
        Phrase::ChallengeTierPreview.text(lang),
        tokens::MUTED,
    );
    let tiers = row(commands, section, kit);
    let count = model
        .tower
        .badges
        .iter()
        .find(|badge| badge.kind == kind)
        .map_or(0, |badge| badge.count);
    for (tier, threshold) in [
        api::Tier::Bronze,
        api::Tier::Silver,
        api::Tier::Gold,
        api::Tier::Platinum,
        api::Tier::Obsidian,
    ]
    .into_iter()
    .zip(kind.thresholds())
    {
        let cell = column(commands, tiers, kit, 17.0);
        commands
            .entity(cell)
            .entry::<Node>()
            .and_modify(|mut node| node.align_items = AlignItems::Center);
        medal(commands, cell, kit, kind, Some(tier), 128.0);
        label(
            commands,
            cell,
            kit,
            super::super::rewards::tier(Some(tier)).text(lang),
            tokens::INK,
        );
        let caption = if count >= threshold {
            Phrase::ChallengeTierEarned
        } else {
            Phrase::ChallengeTierGoal
        };
        label(
            commands,
            cell,
            kit,
            &caption.fill(lang, &[&threshold.to_string()]),
            tokens::MUTED,
        );
    }
}

fn name(kind: BadgeKind) -> Phrase {
    match kind {
        BadgeKind::Pathfinder => Phrase::TowerPathfinder,
        BadgeKind::Summit => Phrase::TowerSummit,
        BadgeKind::Unbroken => Phrase::TowerUnbroken,
    }
}

pub(super) fn medal(
    commands: &mut Commands,
    parent: Entity,
    kit: Kit,
    kind: BadgeKind,
    tier: Option<api::Tier>,
    size: f32,
) {
    let icon = commands
        .spawn((
            crate::insignia::Insignia {
                kind: crate::insignia::Motif::Tower(kind),
                tier,
            },
            Node {
                width: kit.m.px(size),
                height: kit.m.px(size),
                align_self: AlignSelf::Center,
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(parent).add_child(icon);
}
