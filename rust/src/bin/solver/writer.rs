use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::mpsc;

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};

use crate::{result::SimulationResult, utils::set_pb_msg};

pub(crate) enum WriterMsg {
    Started { sim_id: usize, start_ms: u128 },
    Finished(SimulationResult),
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
pub(crate) fn spawn_writer_thread(
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

/// The writer terminates once all Sender instances are dropped.
pub(crate) fn writer_loop(
    pb: ProgressBar,
    mut file: File,
    rx: mpsc::Receiver<WriterMsg>,
) -> Result<()> {
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


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    use crate::result::SimulationResult;

    // Minimal dummy SimulationResult
    fn dummy_sim_result(sim_id: usize) -> SimulationResult {
        SimulationResult::new(sim_id, 123, vec![], 0, 0)
    }

    #[test]
    fn spawn_writer_thread_creates_file_and_accepts_messages() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("out.jsonl");

        let (tx, handle) = spawn_writer_thread(2, &file_path).unwrap();

        // Send Started messages
        tx.send(WriterMsg::Started { sim_id: 1, start_ms: 0 }).unwrap();
        tx.send(WriterMsg::Started { sim_id: 2, start_ms: 10 }).unwrap();

        // Send Finished messages
        tx.send(WriterMsg::Finished(dummy_sim_result(1))).unwrap();
        tx.send(WriterMsg::Finished(dummy_sim_result(2))).unwrap();

        // Drop sender so the writer thread exits
        drop(tx);

        let result = handle.join().unwrap();
        assert!(result.is_ok());

        // File should exist and contain two lines
        let content = std::fs::read_to_string(file_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);

        // Each line should be valid JSON
        for line in lines {
            let _: SimulationResult = serde_json::from_str(line).unwrap();
        }
    }

    #[test]
    fn writer_loop_handles_failed_message() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("out_fail.jsonl");

        let (tx, handle) = spawn_writer_thread(1, &file_path).unwrap();

        // Send Failed message
        tx.send(WriterMsg::Failed(1, "sim failed".to_string())).unwrap();

        drop(tx);
        let result = handle.join().unwrap();
        assert!(result.is_ok());

        // File should be empty because we only sent Failed
        let content = std::fs::read_to_string(file_path).unwrap();
        assert_eq!(content, "");
    }

    #[test]
    fn writer_loop_accepts_started_and_failed_mix() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("out_mix.jsonl");

        let (tx, handle) = spawn_writer_thread(3, &file_path).unwrap();

        tx.send(WriterMsg::Started { sim_id: 0, start_ms: 0 }).unwrap();
        tx.send(WriterMsg::Failed(0, "failure 0".to_string())).unwrap();
        tx.send(WriterMsg::Finished(dummy_sim_result(1))).unwrap();

        drop(tx);
        let result = handle.join().unwrap();
        assert!(result.is_ok());

        let content = std::fs::read_to_string(file_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 1);

        let _: SimulationResult = serde_json::from_str(lines[0]).unwrap();
    }

    #[test]
    fn spawn_writer_thread_fails_on_invalid_path() {
        // Path to a directory that cannot be created (should fail)
        let invalid_path = Path::new("/this/path/should/not/exist/out.jsonl");
        let res = spawn_writer_thread(1, invalid_path);
        assert!(res.is_err());
    }
}
