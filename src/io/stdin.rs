use crate::io::signals::SigIntAction;
use signal_hook::consts::SIGINT;
use std::io::Write;
use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;

pub(crate) fn handle_stdin(
    child: &mut Child,
    pid: u32,
    suppress_stdin: Arc<AtomicBool>,
    is_confirming: Arc<AtomicBool>,
    action: Arc<RwLock<SigIntAction>>,
) {
    let mut child_stdin = child.stdin.take().expect("Failed to get child stdin");
    thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut input = String::new();

        loop {
            input.clear();
            stdin.read_line(&mut input).expect("Failed to read input");
            if !suppress_stdin.load(Ordering::SeqCst) {
                child_stdin
                    .write_all(input.as_bytes())
                    .expect("Failed to write to child stdin");
                child_stdin.flush().expect("Failed to flush child stdin");
            } else {
                let mut action = action.write().unwrap();
                if input.trim().eq("Y") {
                    println!("Terminating command...");
                    unsafe {
                        libc::kill(pid as i32, SIGINT);
                    }
                    suppress_stdin.store(false, Ordering::SeqCst);
                    *action = SigIntAction::Terminate;
                    break;
                } else if input.trim().eq_ignore_ascii_case("n") || input.trim().is_empty() {
                    println!("Continuing command...");
                    *action = SigIntAction::Continue;
                } else {
                    println!("Invalid input. You must enter 'Y', 'n', 'N' or just press ENTER.");
                    *action = SigIntAction::Invalid;
                }
                // send signal to answer to the confirm message
                is_confirming.store(false, Ordering::SeqCst);
            }
        }
    });
}
