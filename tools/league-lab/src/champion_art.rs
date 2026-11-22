//! Resolve the badge for each occupied place in one global daily player snapshot.
use crate::league::{REVISED, Record, standings};
use std::{
    collections::BTreeMap,
    error::Error,
    io::{self, BufRead, Write},
};

const HEADER: &str = "account_id,elo,games,first_day,last_day";

pub fn publish(day: &str) -> Result<(), Box<dyn Error>> {
    snapshot(day.parse()?, io::stdin().lock(), io::stdout().lock())
}

fn snapshot(day: u16, input: impl BufRead, mut output: impl Write) -> Result<(), Box<dyn Error>> {
    let mut rows = input.lines();
    if rows.next().transpose()?.as_deref() != Some(HEADER) {
        return Err(format!("expected header {HEADER}").into());
    }
    let mut accounts = BTreeMap::new();
    for row in rows {
        let row = row?;
        let fields: Vec<_> = row.split(',').collect();
        let [id, elo, games, first, last] = fields.as_slice() else {
            return Err(format!("expected columns {HEADER}").into());
        };
        let record = Record {
            elo: elo.parse()?,
            games: games.parse()?,
            first: first.parse()?,
            last: last.parse()?,
        };
        if record.first > record.last || record.last > day {
            return Err(format!("invalid match dates for account {id}").into());
        }
        if accounts.insert(id.parse::<u64>()?, record).is_some() {
            return Err(format!("duplicate account {id}").into());
        }
    }
    // Stable account order is the existing standings tie-break.
    let (ids, records): (Vec<_>, Vec<_>) = accounts.into_iter().unzip();
    writeln!(output, "place,account_id,elo,player_badge")?;
    for (index, id) in standings(&records, REVISED, day)
        .champions
        .into_iter()
        .enumerate()
    {
        let place = index + 1;
        let badge = match place {
            1 => "champion-1",
            2 => "champion-2",
            3 => "champion-3",
            _ => "champion",
        };
        writeln!(
            output,
            "{place},{},{},player/{badge}.png",
            ids[id], records[id].elo
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn board(day: u16, rows: &str) -> String {
        let mut out = Vec::new();
        snapshot(day, format!("{HEADER}\n{rows}").as_bytes(), &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }
    #[test]
    fn podium_moves_with_the_daily_snapshot_and_leaves_vacancies() {
        let base =
            "40,2600,3000,0,199\n30,2700,3000,0,200\n20,2700,3000,0,211\n10,2800,3000,0,199\n";
        let today = board(211, base);
        assert!(today.ends_with("1,10,2800,player/champion-1.png\n2,20,2700,player/champion-2.png\n3,30,2700,player/champion-3.png\n4,40,2600,player/champion.png\n"));
        let later = board(214, base);
        assert!(
            later.ends_with("1,20,2700,player/champion-1.png\n2,30,2700,player/champion-2.png\n")
        );
        assert_eq!(later.lines().count(), 3);
        assert_eq!(board(226, base).lines().count(), 1);
    }
}
