//! The table the player sits at, as the renderer draws it (§10.2): the
//! roster the gateway's member list is checked against (§5.4), the `@`
//! candidates, and where recent players come from.

use baylee_protocol::social::{Conv, valid_tag};

/// One chair of the roster, as the table draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RosterSeat {
    /// The seat number.
    pub seat: u8,
    /// The handle drawn (`Bob#0012`); an AI chair's name has no tag.
    pub handle: String,
    /// The team, at a table with teams.
    pub team: Option<u8>,
}

impl RosterSeat {
    /// The tag the handle ends in, if it is a person's handle.
    #[must_use]
    pub fn tag(&self) -> Option<&str> {
        tag_of(&self.handle)
    }
}

/// The tag a handle (`Name#tag`) ends in.
#[must_use]
pub fn tag_of(handle: &str) -> Option<&str> {
    handle
        .rsplit_once('#')
        .map(|(_, tag)| tag)
        .filter(|tag| valid_tag(tag))
}

/// Where the player is at a table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableContext {
    /// The lobby game's id (the `tbl:` conversation's).
    pub game: String,
    /// The game is running (else the waiting room).
    pub playing: bool,
    /// Every chair as drawn.
    pub roster: Vec<RosterSeat>,
    /// This player's seat; `None` while watching.
    pub me_seat: Option<u8>,
    /// This player's team, at a table with teams.
    pub my_team: Option<u8>,
    /// Watching, not seated: no table chat (§5.5).
    pub watching: bool,
}

impl TableContext {
    /// The table's conversation, when seated.
    #[must_use]
    pub fn table_conv(&self) -> Option<Conv> {
        (!self.watching).then(|| Conv::Table {
            game: self.game.clone(),
        })
    }

    /// The team's conversation, when seated at a table with teams.
    #[must_use]
    pub fn team_conv(&self) -> Option<Conv> {
        match (self.watching, self.my_team) {
            (false, Some(team)) => Some(Conv::Team {
                game: self.game.clone(),
                team,
            }),
            _ => None,
        }
    }

    /// Whether the roster shows `tag` in a chair (of `team`, when given).
    #[must_use]
    pub fn seats(&self, tag: &str, team: Option<u8>) -> bool {
        self.roster
            .iter()
            .any(|s| s.tag() == Some(tag) && (team.is_none() || s.team == team))
    }

    /// The people in the roster (chairs with a handle), but `me`.
    pub fn people<'a>(&'a self, me: &'a str) -> impl Iterator<Item = &'a RosterSeat> + 'a {
        self.roster
            .iter()
            .filter(move |s| s.tag().is_some_and(|t| t != me))
    }
}
