use signal_hook::consts::SIGINT;
use std::io;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;

use super::SigIntAction;

pub(crate) fn handle_sigint(
    pid: u32,
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
        print!("\x1b[1;31mCtrl-C detected. Do you want to terminate the command? (Y/[n]): \x1b[0m");
        io::stdout()
            .flush()
            .expect("\x1b[31mFailed to flush stdout\x1b[0m");
        while is_confirming.load(Ordering::SeqCst) {
            thread::sleep(std::time::Duration::from_millis(100));
        }
        match *action.read().unwrap() {
            SigIntAction::Continue => {
                suppress_output.store(false, Ordering::SeqCst);
                break;
            }
            SigIntAction::Terminate => {
                unsafe {
                    libc::kill(pid as i32, SIGINT);
                }
                println!("\x1b[32mProgram terminated.\x1b[0m");
                break;
            }
            SigIntAction::Invalid => {
                is_confirming.store(true, Ordering::SeqCst);
                continue;
            }
        }
    }
}
