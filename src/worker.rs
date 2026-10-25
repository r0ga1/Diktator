//! Supervised native inference. The UI/capture process never loads ONNX DLLs.
//! A stalled/crashed worker can be discarded without restarting the application.
use crate::asr::{self, Recognizer};
use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Child, Command, Stdio},
    sync::{atomic::AtomicBool, mpsc},
    thread,
    time::Duration,
};
#[derive(Serialize, Deserialize)]
struct Reply {
    text: String,
    error: String,
}
type Outcome = Result<String, String>;
struct Job {
    pcm: Vec<f32>,
    reply: mpsc::SyncSender<Outcome>,
}
pub struct Worker {
    child: Child,
    jobs: Option<mpsc::SyncSender<Job>>,
    io: Option<thread::JoinHandle<()>>,
    timeout: Duration,
    failed: bool,
}
fn read_reply(reader: &mut impl BufRead) -> Outcome {
    let mut bytes = Vec::new();
    reader
        .take(4 * 1024 * 1024 + 1)
        .read_until(b'\n', &mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.is_empty() || bytes.len() > 4 * 1024 * 1024 {
        return Err("Erkennungsprozess wurde beendet oder lieferte eine ungültige Antwort.".into());
    }
    let r: Reply =
        serde_json::from_slice(&bytes).map_err(|_| "Ungültige Antwort der Erkennung.")?;
    if r.error.is_empty() {
        Ok(r.text)
    } else {
        Err(r.error)
    }
}
impl Worker {
    pub fn load(root: &Path, threads: usize) -> Result<Self, String> {
        use std::os::windows::process::CommandExt;
        let mut command = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
        command
            .arg("--asr-worker")
            .arg(root)
            .arg(threads.to_string())
            .arg(std::process::id().to_string())
            .creation_flags(0x08000000 | 0x00004000);
        Self::spawn(command, Duration::from_secs(90), Duration::from_secs(120))
    }
    fn spawn(mut command: Command, startup: Duration, timeout: Duration) -> Result<Self, String> {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Erkennungsprozess konnte nicht starten: {e}"))?;
        let mut input = child.stdin.take().unwrap();
        let output = child.stdout.take().unwrap();
        let (jobs, receiver) = mpsc::sync_channel::<Job>(1);
        let (ready, startup_reply) = mpsc::sync_channel(1);
        let io = thread::spawn(move || {
            let mut output = BufReader::new(output);
            let initial = read_reply(&mut output);
            let ok = initial.is_ok();
            let _ = ready.send(initial);
            if !ok {
                return;
            }
            while let Ok(job) = receiver.recv() {
                let result = (|| {
                    let mut bytes = Vec::with_capacity(4 + job.pcm.len() * 4);
                    bytes.extend_from_slice(&(job.pcm.len() as u32).to_le_bytes());
                    for sample in job.pcm {
                        bytes.extend_from_slice(&sample.to_le_bytes());
                    }
                    input
                        .write_all(&bytes)
                        .and_then(|_| input.flush())
                        .map_err(|e| e.to_string())?;
                    read_reply(&mut output)
                })();
                let failed = result.is_err();
                let _ = job.reply.send(result);
                if failed {
                    break;
                }
            }
        });
        let mut worker = Self {
            child,
            jobs: Some(jobs),
            io: Some(io),
            timeout,
            failed: false,
        };
        match startup_reply.recv_timeout(startup) {
            Ok(Ok(_)) => Ok(worker),
            Ok(Err(e)) => {
                worker.stop();
                Err(e)
            }
            Err(_) => {
                worker.stop();
                Err("Erkennung antwortet beim Laden nicht. Prozess wurde beendet; bitte erneut diktieren.".into())
            }
        }
    }
    pub fn recognize(&mut self, pcm: &[f32]) -> Outcome {
        if pcm.is_empty() {
            return Ok(String::new());
        }
        if pcm.len() > 16000 * 95 {
            return Err("ASR-Segment ist zu lang.".into());
        }
        if self.failed {
            return Err("Erkennungsprozess wird für das nächste Diktat neu gestartet.".into());
        }
        let (reply, result) = mpsc::sync_channel(1);
        if self
            .jobs
            .as_ref()
            .unwrap()
            .send(Job {
                pcm: pcm.to_vec(),
                reply,
            })
            .is_err()
        {
            self.stop();
            return Err("Erkennungsprozess wurde unerwartet beendet.".into());
        }
        match result.recv_timeout(self.timeout) {
            Ok(Ok(text)) => Ok(text),
            Ok(Err(e)) => {
                self.stop();
                Err(format!("Erkennung wurde beendet: {e}"))
            }
            Err(_) => {
                self.stop();
                Err("Erkennung hat zu lange nicht geantwortet und wurde beendet. Bitte erneut diktieren.".into())
            }
        }
    }
    pub fn recognize_full(&mut self, pcm: &[f32], cancel: &AtomicBool) -> Outcome {
        asr::recognize_chunks(pcm, cancel, |chunk| self.recognize(chunk))
    }
    pub fn failed(&self) -> bool {
        self.failed
    }
    fn stop(&mut self) {
        self.failed = true;
        self.jobs.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(io) = self.io.take() {
            let _ = io.join();
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.stop();
    }
}
/// Private pipe protocol: little-endian sample count followed by f32 mono PCM.
/// Only the parent passes in a verified installed model/runtime directory.
pub fn run(root: &Path, threads: usize, parent_pid: u32) -> Result<(), String> {
    use windows_sys::Win32::{Foundation::*, System::Threading::*};
    let parent = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, parent_pid) };
    if parent.is_null() {
        return Err("Übergeordneter Prozess fehlt.".into());
    }
    let handle = parent as isize;
    thread::spawn(move || unsafe {
        WaitForSingleObject(handle as HANDLE, INFINITE);
        CloseHandle(handle as HANDLE);
        std::process::exit(0);
    });
    let mut output = std::io::stdout().lock();
    fn reply(output: &mut impl Write, result: Outcome) -> Result<(), String> {
        let r = match result {
            Ok(text) => Reply {
                text,
                error: String::new(),
            },
            Err(error) => Reply {
                text: String::new(),
                error,
            },
        };
        serde_json::to_writer(&mut *output, &r).map_err(|e| e.to_string())?;
        output
            .write_all(b"\n")
            .and_then(|_| output.flush())
            .map_err(|e| e.to_string())
    }
    let mut recognizer = match Recognizer::load(&root.join("runtime"), &root.join("model"), threads)
    {
        Ok(r) => {
            reply(&mut output, Ok(String::new()))?;
            r
        }
        Err(e) => {
            reply(&mut output, Err(e.clone()))?;
            return Err(e);
        }
    };
    let mut input = std::io::stdin().lock();
    loop {
        let mut size = [0; 4];
        if let Err(e) = input.read_exact(&mut size) {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                return Ok(());
            }
            return Err(e.to_string());
        }
        let count = u32::from_le_bytes(size) as usize;
        if count > 16000 * 95 {
            return Err("ASR-Pipe-Limit überschritten.".into());
        }
        let mut bytes = vec![0; count * 4];
        input.read_exact(&mut bytes).map_err(|e| e.to_string())?;
        let pcm = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b))
            .collect::<Vec<_>>();
        if pcm.iter().any(|v| !v.is_finite()) {
            return Err("Ungültiges Audio.".into());
        }
        reply(&mut output, recognizer.recognize(&pcm))?;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hung_worker_is_terminated_and_pipe_thread_reaped() {
        let Some(path) = std::env::var_os("DIKTATOR_TEST_HUNG_WORKER") else {
            return;
        };
        let now = std::time::Instant::now();
        let mut worker = Worker::spawn(
            Command::new(path),
            Duration::from_secs(10),
            Duration::from_millis(150),
        )
        .unwrap();
        assert!(worker
            .recognize(&[0.; 1600])
            .unwrap_err()
            .contains("zu lange"));
        assert!(worker.failed());
        assert!(worker.child.try_wait().unwrap().is_some());
        assert!(worker.io.is_none());
        assert!(now.elapsed() < Duration::from_secs(12));
    }
}
