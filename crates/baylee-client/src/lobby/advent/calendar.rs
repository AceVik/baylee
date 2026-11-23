//! Apply only a response matching its authenticated request and current door.
#[cfg(test)]
mod tests;
use super::{Book, Calendar, Challenge, Screen, api};
use baylee_account::Error;
use std::time::{Duration, Instant};

impl Book {
    pub(super) fn accept(&mut self, request: api::Request, reply: Result<api::Response, Error>) {
        let result = reply.and_then(|reply| self.apply(&request, reply));
        if matches!(request, api::Request::Inventory { .. }) {
            let error = result.as_ref().err().copied();
            if error.is_some() {
                self.inventory_retry = Some(Instant::now() + Duration::from_secs(30));
            }
            if self.inventory_error != error || result == Ok(true) {
                self.touch();
            }
            self.inventory_error = error;
            return;
        }
        if result.is_err()
            && let api::Request::Claim { year, door } = request
            && self.recovery.contains(&(year, door))
        {
            self.recovery.clear();
            self.refresh = Instant::now() + Duration::from_secs(10);
        }
        let error = result.as_ref().err().copied();
        if self.error != error || result == Ok(true) {
            self.touch();
        }
        self.error = error;
    }

    fn apply(&mut self, request: &api::Request, reply: api::Response) -> Result<bool, Error> {
        use api::{Request, Response};
        match (request, reply) {
            (Request::CurrentCalendar {}, Response::NoCalendar {}) => {
                let changed = self.calendar.take().is_some();
                self.refresh = Instant::now() + Duration::from_secs(3600);
                if matches!(self.screen, Screen::Calendar | Screen::Door(_)) {
                    self.screen = Screen::Closed;
                }
                return Ok(changed);
            }
            (Request::CurrentCalendar {}, reply @ Response::Calendar { year, .. }) => {
                return self.apply(&Request::Calendar { year }, reply);
            }
            (
                Request::Calendar { year },
                Response::Calendar {
                    year: actual,
                    starts_at,
                    closes_at,
                    now,
                    doors,
                    hero,
                },
            ) if *year == actual
                && closes_at > starts_at
                && (hero.is_none() || (starts_at..closes_at).contains(&now))
                && doors.len() == 24
                && doors
                    .iter()
                    .enumerate()
                    .all(|(i, d)| usize::from(d.door) == i + 1) =>
            {
                return Ok(self.apply_calendar(actual, starts_at, closes_at, now, doors, hero));
            }
            (
                Request::Puzzle { year, door },
                Response::Puzzle {
                    year: actual,
                    door: from,
                    puzzle,
                },
            ) if *year == actual && *door == from => {
                puzzle.validate()?;
                if self.screen == Screen::Door(from) && self.year() == actual {
                    self.puzzle = Some(Challenge {
                        door: from,
                        puzzle,
                        line: api::puzzle::Line(Vec::new()),
                        hints: [None, None],
                        submitted: None,
                    });
                } else {
                    return Ok(false);
                }
            }
            (
                Request::Hint { year, door, number },
                Response::Hint {
                    number: actual,
                    text,
                },
            ) if actual == *number && matches!(actual, 1 | 2) => {
                if self.year() == *year
                    && self.screen == Screen::Door(*door)
                    && let Some(challenge) = self.puzzle.as_mut().filter(|p| p.door == *door)
                {
                    challenge.hints[usize::from(actual - 1)] = Some(text);
                } else {
                    return Ok(false);
                }
            }
            (Request::Submit { year, door, line }, Response::Submission { won }) => {
                if self.year() != *year {
                    return Ok(false);
                }
                if let Some(challenge) = self
                    .puzzle
                    .as_mut()
                    .filter(|p| p.door == *door && &p.line == line)
                {
                    challenge.submitted = Some(won);
                }
                if won {
                    self.mark_door(*door, api::DoorState::Claimed);
                    self.request_inventory();
                    self.push(Request::Calendar { year: self.year() });
                }
            }
            (Request::Claim { year, door }, Response::Reward { reward })
                if reward.year == *year && reward.door == *door =>
            {
                self.receive_reward(reward);
            }
            (Request::Inventory { after }, Response::Inventory { items, next }) => {
                return self.receive_inventory(*after, items, next);
            }
            _ => return Err(Error::Rejected),
        }
        Ok(true)
    }

    fn apply_calendar(
        &mut self,
        year: u16,
        starts_at: i64,
        closes_at: i64,
        now: i64,
        doors: Vec<api::Door>,
        hero: Option<api::AssetMeta>,
    ) -> bool {
        let same = self.calendar.as_ref().is_some_and(|c| {
            c.year == year
                && c.doors == doors
                && c.hero == hero
                && c.opens == starts_at
                && c.closes == closes_at
                && self.released() == (starts_at..closes_at).contains(&now)
        });
        let original_screen = self.screen;
        let boundary = doors
            .iter()
            .map(|d| d.enables_at)
            .chain([starts_at, closes_at])
            .filter(|&t| t > now)
            .min();
        let seconds = boundary.map_or(3600, |t| t.saturating_sub(now).clamp(1, 3600));
        self.refresh = Instant::now() + Duration::from_secs(u64::try_from(seconds).unwrap_or(60));
        self.calendar = Some(Calendar {
            year,
            opens: starts_at,
            closes: closes_at,
            as_of: now,
            observed: Instant::now(),
            doors,
            hero,
        });
        if self.recovery.is_empty() {
            self.recovery = self
                .calendar
                .iter()
                .flat_map(|c| &c.doors)
                .filter(|d| {
                    d.state == api::DoorState::Solved && self.screen != Screen::Door(d.door)
                })
                .map(|d| (year, d.door))
                .collect();
            if let Some(&(year, door)) = self.recovery.last() {
                self.push(api::Request::Claim { year, door });
            }
        }
        if !self.released() && matches!(self.screen, Screen::Calendar | Screen::Door(_)) {
            self.screen = Screen::Closed;
        }
        self.load_hero();
        !same || self.screen != original_screen
    }
    fn receive_reward(&mut self, reward: api::Reward) {
        let (year, door) = (reward.year, reward.door);
        let recovery = self.recovery.contains(&(year, door));
        self.recovery.retain(|&key| key != (year, door));
        if self.year() == year {
            self.mark_door(door, api::DoorState::Claimed);
        }
        if !recovery && self.year() == year && self.screen == Screen::Door(door) {
            self.screen = Screen::Item(reward.id);
            self.media
                .want(api::AssetScope::Owned { item: reward.id }, reward.asset);
        }
        self.inventory.retain(|item| item.id != reward.id);
        if self
            .work
            .iter()
            .any(|request| matches!(request, api::Request::Inventory { after: Some(_) }))
            && !self.inventory_sync.iter().any(|item| item.id == reward.id)
        {
            self.inventory_sync.insert(0, reward.clone());
        }
        self.inventory.insert(0, reward);
        if recovery {
            if let Some(&(year, door)) = self.recovery.last() {
                self.push(api::Request::Claim { year, door });
            } else {
                self.request_inventory();
            }
        } else {
            self.request_inventory();
            self.push(api::Request::CurrentCalendar {});
        }
    }
    fn receive_inventory(
        &mut self,
        after: Option<api::Id>,
        items: Vec<api::Reward>,
        next: Option<api::Id>,
    ) -> Result<bool, Error> {
        if after.is_none() {
            self.inventory_sync.clear();
        }
        for item in items {
            if !self.inventory_sync.iter().any(|v| v.id == item.id) {
                self.inventory_sync.push(item);
            }
        }
        if let Some(cursor) = next {
            if after == Some(cursor) {
                return Err(Error::Conflict);
            }
            self.push(api::Request::Inventory {
                after: Some(cursor),
            });
            return Ok(false);
        }

        let changed = !self.inventory_loaded || self.inventory != self.inventory_sync;
        self.inventory = std::mem::take(&mut self.inventory_sync);
        self.inventory_loaded = true;
        self.inventory_retry = None;
        if let Screen::Item(id) = self.screen {
            self.load_owned(id);
        }
        Ok(changed)
    }
    fn mark_door(&mut self, n: u8, state: api::DoorState) {
        if let Some(door) = self
            .calendar
            .as_mut()
            .and_then(|c| c.doors.iter_mut().find(|d| d.door == n))
        {
            door.state = state;
        }
    }
    pub(super) fn load_hero(&mut self) {
        if self.screen == Screen::Calendar
            && self.released()
            && let Some((year, meta)) = self.calendar.as_ref().and_then(|c| Some((c.year, c.hero?)))
        {
            self.media.want(api::AssetScope::Calendar { year }, meta);
        }
    }
    pub(super) fn load_owned(&mut self, id: api::Id) {
        if let Some(reward) = self.inventory.iter().find(|r| r.id == id) {
            self.media
                .want(api::AssetScope::Owned { item: id }, reward.asset);
        }
    }
    pub(super) fn retry(&mut self) {
        match self.screen {
            Screen::Door(door) => self.open_door(door),
            Screen::Item(id) => {
                self.load_owned(id);
                self.request_inventory();
            }
            Screen::Gallery => self.request_inventory(),
            Screen::Calendar => {
                self.load_hero();
                self.push(api::Request::CurrentCalendar {});
            }
            Screen::Closed => {}
        }
    }
    pub(super) fn open_door(&mut self, door: u8) {
        let Some(state) = self.door(door).map(|d| d.state) else {
            return;
        };
        if matches!(state, api::DoorState::Locked | api::DoorState::Expired) {
            return;
        }
        self.screen = Screen::Door(door);
        self.puzzle = None;
        self.inventory_page = 0;
        match state {
            api::DoorState::Open => self.push(api::Request::Puzzle {
                year: self.year(),
                door,
            }),
            api::DoorState::Claimed => {
                if let Some(id) = self
                    .inventory
                    .iter()
                    .find(|r| r.year == self.year() && r.door == door)
                    .map(|r| r.id)
                {
                    self.screen = Screen::Item(id);
                    self.load_owned(id);
                } else {
                    self.request_inventory();
                }
            }
            _ => {}
        }
    }
}
