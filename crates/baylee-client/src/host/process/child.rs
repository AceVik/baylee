//! The private helper owns all local rules and AI work. Its input reader keeps
//! observing owner death even while a rules operation does not return.
use super::wire::{
    Bootstrap, Contract, FRAME_LIMIT, INITIAL_LIMIT, Initial, Input, Output, RECORD_CHUNK, Reply,
};
use crate::host::{DuelHost, LocalHost};
use baylee_core::{preset::LoopPolicy, wire};
use baylee_engine::engine::{DetectLoops, LoopDetection, NoLoopDetection};
use std::{io, sync::mpsc};

pub(super) fn run() -> Result<(), String> {
    let (initial_tx, initial_rx) = mpsc::sync_channel(1);
    let (input_tx, input_rx) = mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("baylee-local-owner".into())
        .spawn(move || {
            let mut stdin = io::stdin().lock();
            let Ok(bytes) = baylee_worker::frame::read(&mut stdin, INITIAL_LIMIT) else {
                std::process::exit(1);
            };
            let Ok(initial) = wire::decode::<Bootstrap>(&bytes) else {
                std::process::exit(1);
            };
            drop(bytes);
            if initial_tx.send(initial).is_err() {
                std::process::exit(1);
            }
            loop {
                let input = baylee_worker::frame::read(&mut stdin, FRAME_LIMIT)
                    .ok()
                    .and_then(|bytes| wire::decode::<Input>(&bytes).ok());
                // Only one operation can be in flight. Refuse flooding instead
                // of blocking this liveness reader behind an unresponsive engine.
                if input.is_none_or(|input| input_tx.try_send(input).is_err()) {
                    std::process::exit(1);
                }
            }
        })
        .map_err(|_| "the local input worker could not start")?;
    let initial = initial_rx
        .recv()
        .map_err(|_| "the local owner disappeared")?;
    let preset = initial.preset()?;
    if !initial.contract.accepts(preset.house_rules.loop_policy)
        || usize::from(initial.seat.get()) >= preset.seats.len()
        || initial.names.len() != preset.seats.len()
    {
        return Err("the local engine contract differs from its owner".into());
    }
    let mut stdout = io::stdout().lock();
    write(
        &mut stdout,
        &Output::Hello(Contract::new(preset.house_rules.loop_policy)),
    )?;
    let host = match preset.house_rules.loop_policy {
        LoopPolicy::Off => make::<NoLoopDetection>(initial, &preset)?,
        LoopPolicy::RunOnceThenBreak | LoopPolicy::CompRulesDraw => {
            make::<DetectLoops>(initial, &preset)?
        }
    };
    play(host, &input_rx, &mut stdout)
}

fn make<D: LoopDetection>(
    initial: Bootstrap,
    preset: &baylee_core::preset::GamePreset,
) -> Result<Box<dyn DuelHost>, String> {
    let resuming = matches!(initial.initial, Initial::Resume { .. });
    let mut host = match initial.initial {
        Initial::New(_) => {
            let names: Vec<_> = initial.names.iter().map(String::as_str).collect();
            LocalHost::<D>::with_detection(preset, initial.seat, &names)
                .ok_or("the local game setup is not playable")?
        }
        Initial::Resume { record, name, hash } => {
            let host =
                LocalHost::<D>::resume_with_detection(record, &name, initial.seat, &initial.names)?;
            if host.hash() != hash {
                return Err("the rebuilt game differs from the restart".into());
            }
            host
        }
    };
    // Reconstruct and verify first, with the store still closed. In particular
    // a wrong restart hash must not rewrite the player's existing tape on Drop.
    if initial.persist {
        crate::settings::open_store();
        host.live = if resuming {
            crate::records::LiveRecord::continuing(&host.record_name, host.record.len())
        } else {
            crate::records::LiveRecord::new(&host.record_name)
        };
        host.live.write(&host.record);
    }
    Ok(Box::new(host.searching()))
}

fn play(
    mut host: Box<dyn DuelHost>,
    input: &mpsc::Receiver<Input>,
    writer: &mut impl io::Write,
) -> Result<(), String> {
    let mut recorded = 0;
    loop {
        let messages = host.poll();
        let done = !host.busy();
        let record = host.local_record().unwrap_or_default();
        // A large resumed tape is streamed in bounded pieces, never one giant
        // response allocation. Every byte is forwarded exactly once.
        for chunk in record[recorded..].chunks(RECORD_CHUNK) {
            write(
                writer,
                &Output::Progress(Reply {
                    messages: Vec::new(),
                    record: chunk.to_vec(),
                    resume: None,
                    done: false,
                }),
            )?;
        }
        recorded = record.len();
        write(
            writer,
            &Output::Progress(Reply {
                messages,
                record: Vec::new(),
                resume: done.then(|| host.resume_point()).flatten(),
                done,
            }),
        )?;
        if done {
            match input.recv().map_err(|_| "the local owner disappeared")? {
                Input::Action(action) => host.submit(action),
                Input::SeatSetting(setting) => host.seat_setting(setting),
            }
        }
    }
}

fn write(writer: &mut impl io::Write, value: &Output) -> Result<(), String> {
    baylee_worker::frame::write(writer, &wire::encode(value), FRAME_LIMIT)
        .map_err(|_| "the local engine output pipe failed".into())
}
