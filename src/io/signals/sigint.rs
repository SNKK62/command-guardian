use std::io;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;

use super::SigIntAction;

pub(crate) fn handle_sigint(
    suppress_output: Arc<AtomicBool>,
    is_confirming: Arc<AtomicBool>,
    action: Arc<RwLock<SigIntAction>>,
) {
    suppress_output.store(true, Ordering::SeqCst);
    is_confirming.store(true, Ordering::SeqCst);
    let mut is_first = true;
    loop {
        if is_first {
            println!();
            is_first = false;
        }
        print!("Ctrl-C detected. Do you want to terminate the command? (Y/[n]):");
        io::stdout().flush().expect("Failed to flush stdout");
        while is_confirming.load(Ordering::SeqCst) {
            thread::sleep(std::time::Duration::from_millis(100));
        }
        match *action.read().unwrap() {
            SigIntAction::Continue => {
                suppress_output.store(false, Ordering::SeqCst);
                break;
            }
            SigIntAction::Terminate => {
                println!("Program terminated.");
                break;
            }
            SigIntAction::Invalid => {
                is_confirming.store(true, Ordering::SeqCst);
                continue;
            }
        }
    }
}
