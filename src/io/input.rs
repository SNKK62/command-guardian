use crate::io::signals::SigIntAction;
use std::io::Write;
use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;

pub(crate) fn handle_stdin(
    child: &mut Child,
    suppress_stdin: Arc<AtomicBool>,
    is_confirming: Arc<AtomicBool>,
    action: Arc<RwLock<SigIntAction>>,
) {
    let mut child_stdin = child
        .stdin
        .take()
        .expect("\x1b[31mFailed to get child stdin\x1b[0m");
    thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut input = String::new();

        loop {
            input.clear();
            stdin
                .read_line(&mut input)
                .expect("\x1b[31mFailed to read input\x1b[0m");
            if !suppress_stdin.load(Ordering::SeqCst) {
                child_stdin
                    .write_all(input.as_bytes())
                    .expect("\x1b[31mFailed to write to child stdin\x1b[0m");
                child_stdin
                    .flush()
                    .expect("\x1b[31mFailed to flush child stdin\x1b[0m");
            } else {
                let mut action = action.write().unwrap();
                if input.trim().eq("Y") {
                    // set false to suppress_stdin to render the output
                    // of the child process on keyboard interruption.
                    // actually, suppress_stdin is the same as suppress_output.
                    suppress_stdin.store(false, Ordering::SeqCst);
                    // set false to is_confirming to exec handle_sigint
                    is_confirming.store(false, Ordering::SeqCst);
                    *action = SigIntAction::Terminate;
                    break;
                } else if input.trim().eq_ignore_ascii_case("n") || input.trim().is_empty() {
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
