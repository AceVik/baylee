//! The cursor grid and picking an ability, a choice or a cast row.

#[allow(clippy::wildcard_imports)] // the input module's shared vocabulary
use super::*;

/// Visits the next seat in ring order (`F`), or the previous (`Shift+F`),
/// coming home past either end (TABLE-KEYBOARD §9).
///
/// One key that walks the table rather than a numbered key per chair: a
/// four-seat game has three opponents, and a binding screen listing nine
/// "focus seat N" rows would be listing six that can never fire. Ring order
/// is the layout's slot order — clockwise in turn order from my own seat —
/// so `F` walks the table the way the eye goes round it.
pub(super) fn visit_next_seat(duel: &mut Duel, forward: bool) {
    let (Some(layout), Some(me)) = (duel.layout.as_ref(), duel.seat()) else {
        return;
    };
    let ring: Vec<_> = layout.slots.iter().map(|slot| slot.player).collect();
    match baylee_client_core::tableview::step_seat(&ring, me, duel.visiting, forward) {
        Some(seat) => navigate_to_player(duel, seat),
        // Past the last seat is home again, so the key never dead-ends.
        None => navigate_home(duel),
    }
}

/// The selectable cards as a row grid: hand at the bottom, then each
/// seat's lanes from the local seat outward, then the stack. Row order
/// matches the visual layout, so W/S moves the way the eye expects.
///
/// The stack is last because it is drawn highest — the panel is pinned to the
/// top right — and it is here at all because a spell on the stack is a legal
/// target ("target spell", every counterspell in the game) and the keyboard
/// could not reach one. The grid was built out of the *board*, and the stack
/// is not on the board: the cursor walked the hand and the lanes and simply
/// never arrived, so a `ChooseTargets` naming only a spell was a question
/// with no keyboard answer. The pointer's half of the same gap is the row
/// carrying `HandCardVisual`; this is the other half.
pub(super) fn cursor_grid(duel: &Duel) -> Vec<Vec<ObjectId>> {
    let Some(board) = duel.board.as_ref() else {
        return Vec::new();
    };
    let mut rows: Vec<Vec<ObjectId>> = Vec::new();
    let hand: Vec<ObjectId> = board.hand.iter().map(|c| c.id).collect();
    if !hand.is_empty() {
        rows.push(hand);
    }
    for pod in board
        .pods
        .iter()
        .filter(|p| p.is_local)
        .chain(board.pods.iter().filter(|p| !p.is_local))
    {
        for lane in &pod.lanes {
            // A tucked card (#305) is visited right after its host.
            let row: Vec<ObjectId> = lane
                .groups
                .iter()
                .flat_map(baylee_client_core::board::CardGroup::with_attached)
                .map(|g| g.representative)
                .collect();
            if !row.is_empty() {
                rows.push(row);
            }
        }
    }
    // Top of the stack first, which is the order the panel draws and the
    // order the objects resolve in — so `A`/`D` walks the queue downwards.
    let stack: Vec<ObjectId> = board.stack.iter().map(|item| item.id).collect();
    if !stack.is_empty() {
        rows.push(stack);
    }
    rows
}

/// Moves the card cursor; wraps inside a row and clamps the column when
/// changing rows. With no cursor yet, starts at the first hand card.
/// Walks the keyboard cursor over the hand and the lanes.
///
/// Every arm here clears `hovered_at`, because the keyboard cursor is not on
/// the screen: it names a card, not a place. Left behind, the anchor would
/// still be wherever the pointer last stopped, and the preview would open
/// beside a card the cursor walked away from three presses ago.
pub(super) fn move_cursor(duel: &mut Duel, d_row: i32, d_col: i32) {
    let grid = cursor_grid(duel);
    if grid.is_empty() {
        return;
    }
    let Some(current) = duel.hovered else {
        duel.hovered = Some(grid[0][0]);
        duel.hovered_at = None;
        return;
    };
    let Some((mut row, mut col)) = grid.iter().enumerate().find_map(|(r, row)| {
        row.iter()
            .position(|&id| id == current)
            .map(|c| (r as i32, c as i32))
    }) else {
        duel.hovered = Some(grid[0][0]);
        duel.hovered_at = None;
        return;
    };
    if d_col != 0 {
        let len = grid[row as usize].len() as i32;
        col = (col + d_col).rem_euclid(len);
    }
    if d_row != 0 {
        row = (row + d_row).rem_euclid(grid.len() as i32);
        col = col.min(grid[row as usize].len() as i32 - 1);
    }
    duel.hovered = Some(grid[row as usize][col as usize]);
    duel.hovered_at = None;
}

/// Sends the ability one row of the ability sheet stands for.
///
/// It is [`take_sheet_row`] and nothing else: a click on a row and the digit
/// drawn on that row are the same press, so the row arms on the first and
/// sends on the second exactly as the keyboard does — and the list is rebuilt
/// from `LegalActions` in there rather than trusted from the sheet, because a
/// sheet drawn a frame ago must not be able to send an ability the engine has
/// since stopped offering.
pub(super) fn pick_ability(duel: &mut Duel, index: usize) {
    take_sheet_row(duel, index);
}

/// Answers an indexed choice: a colour, a seat, one of several ways to cast.
///
/// It answers on the click that picks it -- there is no second "OK", because
/// there is nothing to combine. The rows are rebuilt from the *current*
/// prompt first, so a button drawn before the engine moved on answers nothing
/// rather than the wrong thing.
///
/// `pub` for the same reason [`activate_card`] is: a row of this chooser is a
/// press, and a test that built the answer by hand would pass just as loudly
/// with no button behind it.
pub fn pick_choice(duel: &mut Duel, index: usize) {
    // This client's own chooser first, because while it stands it *is* the
    // question on the bar — the engine is still holding an ordinary priority
    // window behind it. See [`take_cast_row`].
    if duel.cast_menu.is_some() {
        take_cast_row(duel, index);
        return;
    }
    if pick_attack_choice(duel, index) {
        return;
    }
    if page_damage_choice(duel, index) {
        return;
    }
    if let Some(i) = duel.interaction.as_mut()
        && matches!(
            i.pending(),
            baylee_engine::choice::Pending::ChooseTargets { .. }
        )
    {
        if let Some(filter) = baylee_client_core::targeting::filter_at(index) {
            duel.target_filter = filter;
            duel.target_page = 0;
            return;
        }
        let options = baylee_client_core::targeting::options(i.pending());
        let visible = duel.view.as_ref().map_or(options.len(), |v| {
            baylee_client_core::targeting::filtered(i.pending(), v, duel.target_filter).len()
        });
        if index == baylee_client_core::targeting::PREVIOUS {
            duel.target_page = duel.target_page.saturating_sub(1);
            return;
        }
        if index == baylee_client_core::targeting::NEXT {
            duel.target_page = (duel.target_page + 1)
                .min(visible.saturating_sub(1) / baylee_client_core::targeting::PAGE_SIZE);
            return;
        }
        match options.get(index) {
            Some(baylee_client_core::targeting::Target::Object(id)) => {
                i.toggle(*id);
            }
            Some(baylee_client_core::targeting::Target::Player(id)) => {
                i.toggle_player(*id);
            }
            None => {}
        }
        return;
    }
    let offered = duel
        .interaction
        .as_ref()
        .map(baylee_client_core::Interaction::prompt)
        // The language and the face names are irrelevant here and
        // deliberately not plumbed: only the *shape* of the answer is read
        // back -- whether this prompt is an indexed choice at all, and how
        // many rows it has. The labels are the renderer's business.
        .and_then(|p| {
            crate::choices::options(
                &p,
                baylee_client_core::Lang::En,
                duel.statics.as_ref(),
                &duel.subtype_filter,
                crate::choices::FaceNames::default(),
            )
        })
        // Not `index < rows.len()`: a filtered list's rows carry the
        // engine's own indices, and most of them are not on screen.
        .is_some_and(|rows| rows.iter().any(|row| row.index == index));
    if !offered {
        return;
    }
    let action = duel.interaction.as_mut().and_then(|i| {
        if !crate::choices::pick(i, index) {
            return None;
        }
        if matches!(
            i.prompt(),
            Prompt::TextReplacement { .. }
                | Prompt::ChooseManaAbility { .. }
                | Prompt::ChooseDamageSource { .. }
                | Prompt::ChooseDamageEffect { .. }
                | Prompt::AllocatePrevention { .. }
        ) {
            return None;
        }
        i.confirm()
    });
    if let Some(action) = action {
        duel.submit(action);
    }
}

/// Damage and source dialogs page independently of ordinary target rows.
fn page_damage_choice(duel: &mut Duel, index: usize) -> bool {
    if let Some(i) = duel.interaction.as_ref().filter(|i| {
        matches!(
            i.decision_id(),
            Some(
                baylee_client_core::interaction::DecisionId::Damage(_)
                    | baylee_client_core::interaction::DecisionId::Source(_)
            )
        )
    }) {
        let count = match i.prompt() {
            Prompt::TextReplacement { .. } => 10,
            Prompt::ChooseManaAbility { options, .. } => options.len(),
            Prompt::ChooseDamageSource { options } => options.len(),
            Prompt::ChooseDamageEffect { options, .. } => options.len(),
            Prompt::AllocatePrevention { damage, .. } => damage.len(),
            _ => 0,
        };
        if index == baylee_client_core::targeting::PREVIOUS {
            duel.target_page = duel.target_page.saturating_sub(1);
            return true;
        }
        if index == baylee_client_core::targeting::NEXT {
            duel.target_page = (duel.target_page + 1)
                .min(count.saturating_sub(1) / crate::choices::DAMAGE_PAGE_SIZE);
            return true;
        }
    }
    false
}

/// Combat rows edit a draft; the ordinary confirmation remains the only send.
fn pick_attack_choice(duel: &mut Duel, index: usize) -> bool {
    use baylee_client_core::targeting::{NEXT, PAGE_SIZE, PREVIOUS};
    let Some(i) = duel.interaction.as_mut() else {
        return false;
    };
    if !matches!(
        i.pending(),
        baylee_engine::choice::Pending::ChooseAttackers { .. }
    ) {
        return false;
    }
    let options = i.attack_options();
    match index {
        PREVIOUS => duel.target_page = duel.target_page.saturating_sub(1),
        NEXT => {
            let creatures = options
                .iter()
                .filter(|o| matches!(o, baylee_client_core::interaction::AttackOption::Toggle(_)))
                .count();
            duel.target_page = (duel.target_page + 1).min(creatures.saturating_sub(1) / PAGE_SIZE);
        }
        _ => {
            if let Some(option) = options.get(index) {
                i.edit_attack(*option);
            }
        }
    }
    true
}

/// Takes one row of the cast chooser: the way is remembered, the deed is
/// armed, the chooser closes.
///
/// It **arms** rather than sending, which is the same two-stage rule the
/// ability sheet's rows follow ([`take_sheet_row`]) and for the same reason:
/// there is no undo in the engine, and a spell on the stack is the least
/// undoable thing in the game. So the press that answers this client's
/// question leaves the deed standing in the prompt bar, and the next one
/// sends it.
///
/// The list is rebuilt from the current `LegalActions` before the row is
/// read, exactly as every other chooser in this file does — a bar drawn a
/// frame ago must not be able to pick a way the board no longer offers.
pub(super) fn take_cast_row(duel: &mut Duel, at: usize) {
    let Some(card) = duel.cast_menu.as_ref().map(|m| m.card) else {
        return;
    };
    let Some(mode) = cast_menu_for(duel, card).and_then(|m| m.mode(at).cloned()) else {
        // The card has stopped offering that many ways. Close rather than
        // guess: the player is looking at a list that is no longer true.
        duel.cast_menu = None;
        duel.last_error = Some(Refusal::Said(Phrase::DeedWithdrawn));
        return;
    };
    duel.cast_menu = None;
    if matches!(
        mode.kind,
        baylee_engine::choice::CastModeKind::PlayLandFace(_)
    ) {
        duel.cast_answer = None;
        arm(duel, card, Deed::Play);
        return;
    }
    duel.cast_answer = Some((card, mode.kind));
    arm(
        duel,
        card,
        Deed::Run {
            plan: mode.plan,
            then: crate::RunEnd::Cast,
        },
    );
}

/// The open cast chooser: the cursor keys walk it, the primary key or confirm
/// takes the row, cancel puts it away. Returns whether it consumed the frame.
///
/// Its own handler rather than a branch of [`ability_menu_keys`], because the
/// two menus are about different things and never stand together — but the
/// same shape, so the keyboard answers this list the way it answers that one.
/// The rows are a single column here, so up and down are the whole of the
/// walk.
///
/// "Never together" is a property of one place and not an observation:
/// [`activate_card`] is the only door either menu is opened through, and it
/// closes the other one before it takes any branch at all.
pub fn cast_menu_keys(fired: Fired, duel: &mut Duel) -> bool {
    let Some(card) = duel.cast_menu.as_ref().map(|m| m.card) else {
        return false;
    };
    let Some(fresh) = cast_menu_for(duel, card) else {
        // Fewer than two ways left: the chooser is stale, and holding it open
        // would keep the keyboard hostage over a question that has answered
        // itself.
        duel.cast_menu = None;
        return false;
    };
    let len = fresh.modes.len();
    if let Some(menu) = duel.cast_menu.as_mut() {
        menu.modes = fresh.modes;
        menu.pick = menu.pick.min(len - 1);
    }
    if fired.has(Action::Cancel) {
        duel.cast_menu = None;
        return true;
    }
    let step = i32::from(fired.has(Action::CursorDown)) - i32::from(fired.has(Action::CursorUp))
        + i32::from(fired.has(Action::CursorRight))
        - i32::from(fired.has(Action::CursorLeft));
    if step != 0 {
        if let Some(menu) = duel.cast_menu.as_mut() {
            let at = i32::try_from(menu.pick).unwrap_or(0);
            let wide = i32::try_from(len).unwrap_or(1);
            menu.pick = usize::try_from((at + step).rem_euclid(wide)).unwrap_or(0);
        }
        return true;
    }
    if fired.has(Action::Primary) || fired.has(Action::Confirm) || fired.has(Action::ActivateCard) {
        let at = duel.cast_menu.as_ref().map_or(0, |m| m.pick);
        take_cast_row(duel, at);
        return true;
    }
    false
}
