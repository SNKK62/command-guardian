use std::io::{BufReader, Read, Write};
use std::os::fd::{FromRawFd, RawFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

pub(crate) fn handle_stderr(master: RawFd, suppress_stderr: Arc<AtomicBool>) {
    let mut stderr_reader = BufReader::new(unsafe { std::fs::File::from_raw_fd(master) });
    thread::spawn(move || {
        let mut buffer = [0; 1024];
        while let Ok(n) = stderr_reader.read(&mut buffer) {
            if n == 0 {
                break;
            }
            if !suppress_stderr.load(Ordering::SeqCst) {
                let output = String::from_utf8_lossy(&buffer[..n]);
                eprint!("{}", output);
                std::io::stdout().flush().unwrap();
            }
        }
    });
}
