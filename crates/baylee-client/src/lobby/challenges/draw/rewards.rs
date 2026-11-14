//! Earned collection and an honest preview of every permanent reward tier.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn badges(commands: &mut Commands, root: Entity, kit: Kit, lang: Lang, model: &State) {
    let collection = panel(commands, root, kit);
    heading(
        commands,
        collection,
        kit,
        Phrase::ChallengeBadges.text(lang),
    );
    let gallery = row(commands, collection, kit);
    commands
        .entity(gallery)
        .entry::<Node>()
        .and_modify(|mut n| n.align_items = AlignItems::Stretch);
    for badge in &model.badges {
        let cell = cell(commands, gallery, kit);
        medal(commands, cell, kit, badge.kind, badge.tier, 112.0);
        let mut small = kit;
        small.m.text = kit.m.small;
        button(
            commands,
            cell,
            small,
            name(badge.kind).text(lang),
            Action::Badge(badge.kind),
            model,
            Weight::Ghost,
        );
        centred(
            commands,
            cell,
            kit,
            tier(badge.tier).text(lang),
            if badge.tier.is_some() {
                tokens::GOLD
            } else {
                tokens::MUTED
            },
        );
        let progress = badge.next.map_or_else(
            || Phrase::ChallengeBadgeMax.fill(lang, &[&badge.count.to_string()]),
            |next| {
                Phrase::ChallengeBadgeCompact
                    .fill(lang, &[&badge.count.to_string(), &next.to_string()])
            },
        );
        centred(commands, cell, kit, &progress, tokens::MUTED);
        meter(commands, cell, kit, badge.count, badge.next);
    }
    if let Some(kind) = model.selected_badge {
        preview(commands, collection, kit, lang, model, kind);
    }
}

pub(super) fn meter(
    commands: &mut Commands,
    cell: Entity,
    kit: Kit,
    count: u32,
    next: Option<u32>,
) {
    let track = commands
        .spawn((
            Node {
                width: percent(85),
                height: kit.m.px(3.0),
                align_self: AlignSelf::Center,
                border_radius: BorderRadius::all(kit.m.px(2.0)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL),
            Pickable::IGNORE,
        ))
        .id();
    let share = next.map_or(1.0, |next| {
        (f64::from(count) / f64::from(next.max(1))).min(1.0)
    });
    let fill = commands
        .spawn((
            Node {
                width: percent((100.0 * share) as f32),
                height: percent(100),
                ..default()
            },
            BackgroundColor(tokens::ACCENT),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(track).add_child(fill);
    commands.entity(cell).add_child(track);
}

fn cell(commands: &mut Commands, root: Entity, kit: Kit) -> Entity {
    let cell = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_basis: kit.m.px(150.0),
                min_width: px_fixed(0.0),
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(6.0),
                padding: UiRect::all(kit.m.px(8.0)),
                border_radius: BorderRadius::all(kit.m.px(10.0)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(cell);
    cell
}

fn medal(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    kind: api::BadgeKind,
    tier: Option<api::Tier>,
    size: f32,
) {
    let icon = commands
        .spawn((
            crate::insignia::Insignia {
                kind: crate::insignia::Motif::Challenge(kind),
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
    commands.entity(root).add_child(icon);
}

fn centred(commands: &mut Commands, root: Entity, kit: Kit, text: &str, ink: Color) {
    let text = commands
        .spawn((
            Text::new(text),
            tf(kit.fonts, kit.m.small),
            TextColor(ink),
            TextLayout::default().with_justify(Justify::Center),
            Node {
                width: percent(100),
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(text);
}

fn preview(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    lang: Lang,
    model: &State,
    kind: api::BadgeKind,
) {
    let section = panel(commands, root, kit);
    heading(commands, section, kit, name(kind).text(lang));
    let requirement = match kind {
        api::BadgeKind::Daily => Phrase::ChallengeEarnDaily,
        api::BadgeKind::Weekly => Phrase::ChallengeEarnWeekly,
        api::BadgeKind::Tactician => Phrase::ChallengeEarnTactician,
        api::BadgeKind::FirstTry => Phrase::ChallengeEarnFirst,
        api::BadgeKind::Regular => Phrase::ChallengeEarnRegular,
    };
    label(commands, section, kit, requirement.text(lang), tokens::INK);
    label(
        commands,
        section,
        kit,
        Phrase::ChallengeTierPreview.text(lang),
        tokens::MUTED,
    );
    let tiers = row(commands, section, kit);
    let count = model
        .badges
        .iter()
        .find(|b| b.kind == kind)
        .map_or(0, |b| b.count);
    for (value, threshold) in [
        api::Tier::Bronze,
        api::Tier::Silver,
        api::Tier::Gold,
        api::Tier::Platinum,
        api::Tier::Obsidian,
    ]
    .into_iter()
    .zip(kind.thresholds())
    {
        let cell = cell(commands, tiers, kit);
        medal(commands, cell, kit, kind, Some(value), 144.0);
        centred(
            commands,
            cell,
            kit,
            tier(Some(value)).text(lang),
            tokens::INK,
        );
        let earned = count >= threshold;
        let caption = if earned {
            Phrase::ChallengeTierEarned.fill(lang, &[&threshold.to_string()])
        } else {
            Phrase::ChallengeTierGoal.fill(lang, &[&threshold.to_string()])
        };
        centred(
            commands,
            cell,
            kit,
            &caption,
            if earned {
                tokens::ACCENT
            } else {
                tokens::MUTED
            },
        );
    }
}

fn name(kind: api::BadgeKind) -> Phrase {
    match kind {
        api::BadgeKind::Daily => Phrase::ChallengeBadgeDaily,
        api::BadgeKind::Weekly => Phrase::ChallengeBadgeWeekly,
        api::BadgeKind::Tactician => Phrase::ChallengeBadgeTactician,
        api::BadgeKind::FirstTry => Phrase::ChallengeBadgeFirst,
        api::BadgeKind::Regular => Phrase::ChallengeBadgeRegular,
    }
}

pub(super) fn tier(tier: Option<api::Tier>) -> Phrase {
    match tier {
        None => Phrase::ChallengeNotEarned,
        Some(api::Tier::Bronze) => Phrase::ChallengeBronze,
        Some(api::Tier::Silver) => Phrase::ChallengeSilver,
        Some(api::Tier::Gold) => Phrase::ChallengeGold,
        Some(api::Tier::Platinum) => Phrase::ChallengePlatinum,
        Some(api::Tier::Obsidian) => Phrase::ChallengeObsidian,
    }
}
