// SPDX-FileCopyrightText: 2026 Lukas Heindl
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! This module implements a writer thread which receives SimulationResults from the worker threads
//! and writes the results in a line-delimited JSON file to disk.
//! As this is the location where information flows together, it is also the responsibility of the
//! writer thread to show some sot of progress indication on the console.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};

use crate::result::SimulationResult;
use crate::utils::RuntimeStats;

/// A type for the communication *worker* -> *writer* thread
pub(super) enum WriterMsg {
    /// Signals the worker started running the simulation
    Started {
        /// the id of the worker which started for tracking the running threads
        sim_id: usize,
    },
    /// Signals the worker finished the simulation, contains the result so the writer can append it
    /// to the output
    Finished(SimulationResult, Duration),
    /// Signals the worker crashed with an error
    Failed(usize, String),
}

/// Spawns the dedicated writer thread.
///
/// The writer thread:
/// - Owns the output file
/// - Serializes [`crate::result::SimulationResult`] as JSON lines
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
        " [{wide_bar}] {pos:>3}/{len:3} | {percent:3}% | {msg} ",
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
fn writer_loop(main_pb: ProgressBar, mut file: File, rx: mpsc::Receiver<WriterMsg>) -> Result<()> {
    let mut stats = RuntimeStats::default();
    main_pb.set_position(0);
    main_pb.set_message(stats.to_string());

    while let Ok(msg) = rx.recv() {
        match msg {
            WriterMsg::Started { sim_id: _sim_id } => {}

            WriterMsg::Finished(sim_res, dur) => {
                let line = serde_json::to_string(&sim_res)?;

                file.write_all(line.as_bytes())?;
                file.write_all(b"\n")?;

                main_pb.inc(1);
                stats.update(dur);
                main_pb.set_message(stats.to_string());
                file.flush()?;
            }

            WriterMsg::Failed(_sim_id, err_msg) => {
                main_pb.inc(1);
                main_pb.println(err_msg);
            }
        }
    }
    main_pb.finish();
    Ok(())
}
