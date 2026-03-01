// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module implements a writer thread which receives SimulationResults from the worker threads
//! and writes the results in a line-delimited JSON file to disk.
//! As this is the location where information flows together, it is also the responsibility of the
//! writer thread to show some sot of progress indication on the console.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::mpsc;

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};

use crate::{result::SimulationResult, utils::set_pb_msg};

/// A type for the communication *worker* -> *writer* thread
pub(super) enum WriterMsg {
    /// Signals the worker started running the simulation
    Started {
        /// the id of the worker which started for tracking the running threads
        sim_id: usize,
        /// the time when the worker started
        start_ms: u128,
    },
    /// Signals the worker finished the simulation, contains the result so the writer can append it
    /// to the output
    Finished(SimulationResult),
    /// Signals the worker crashed with an error
    Failed(usize, String),
}

/// Spawns the dedicated writer thread.
///
/// The writer thread:
/// - Owns the output file
/// - Serializes `SimulationResult` as JSON lines
/// - Maintains and updates a progress bar
/// - Tracks active simulations
///
/// # Arguments
/// - `num_sims` - Total number of simulations expected
/// - `out_path` - Output file path
///
/// # Returns
/// A tuple of:
/// - `Sender<WriterMsg>` for communicating with the writer
/// - `JoinHandle<()>` for joining the thread
pub(super) fn spawn_writer_thread(
    num_sims: usize,
    out_path: &Path,
) -> Result<(mpsc::Sender<WriterMsg>, std::thread::JoinHandle<Result<()>>)> {
    let (tx, rx) = mpsc::channel::<WriterMsg>();

    let pb = ProgressBar::new(num_sims as u64);
    pb.set_style(ProgressStyle::with_template(
        "[{elapsed_precise}] [{wide_bar}] {pos:>3}/{len:3} (ETA: {eta}) {msg}",
    )?);

    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(out_path)?;

    Ok((tx, std::thread::spawn(move || writer_loop(pb, file, rx))))
}

/// This is the "event-loop" of the writer
///
/// It terminates once all Sender instances are dropped.
fn writer_loop(pb: ProgressBar, mut file: File, rx: mpsc::Receiver<WriterMsg>) -> Result<()> {
    let mut active: HashMap<usize, u128> = HashMap::new();

    while let Ok(msg) = rx.recv() {
        match msg {
            WriterMsg::Started { sim_id, start_ms } => {
                active.insert(sim_id, start_ms);
                set_pb_msg(&pb, &active);
            }

            WriterMsg::Finished(sim_res) => {
                let line = serde_json::to_string(&sim_res)?;

                file.write_all(line.as_bytes())?;
                file.write_all(b"\n")?;

                pb.inc(1);
                active.remove(&sim_res.identifier());
                set_pb_msg(&pb, &active);
                file.flush()?;
            }

            WriterMsg::Failed(sim_id, err_msg) => {
                pb.inc(1);
                active.remove(&sim_id);
                pb.println(err_msg);
                set_pb_msg(&pb, &active);
            }
        }
    }
    pb.finish_with_message("done");
    Ok(())
}
