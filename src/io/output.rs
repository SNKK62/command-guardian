use std::io::{BufReader, Read, Write};
use std::os::fd::{FromRawFd, RawFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use super::OutputType;

pub(crate) fn handle_output(
    master: RawFd,
    suppress_output: Arc<AtomicBool>,
    output_type: OutputType,
) {
    let mut reader = BufReader::new(unsafe { std::fs::File::from_raw_fd(master) });
    thread::spawn(move || {
        let mut buffer = [0; 1024];
        while let Ok(n) = reader.read(&mut buffer) {
            if n == 0 {
                break;
            }
            if !suppress_output.load(Ordering::SeqCst) {
                let output = String::from_utf8_lossy(&buffer[..n]);
                match output_type {
                    OutputType::Stdout => {
                        print!("{}", output);
                        std::io::stdout().flush().unwrap();
                    }
                    OutputType::Stderr => {
                        eprint!("{}", output);
                        std::io::stderr().flush().unwrap();
                    }
                };
            }
        }
    });
}
