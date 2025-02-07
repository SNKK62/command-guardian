use signal_hook::consts::SIGINT;
use signal_hook::iterator::Signals;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};
use std::thread;

mod sigint;

pub(super) enum SigIntAction {
    Continue,
    Terminate,
    Invalid,
}

pub(super) fn handle_signals(
    pid: u32,
    suppress_output: Arc<AtomicBool>,
    is_confirming: Arc<AtomicBool>,
    action: Arc<RwLock<SigIntAction>>,
) {
    thread::spawn(move || {
        #[allow(clippy::needless_borrows_for_generic_args)]
        let mut signals =
            Signals::new(&[SIGINT]).expect("\x1b[31mFailed to set up signal handling\x1b[0m");
        for signal in signals.forever() {
            match signal {
                SIGINT => sigint::handle_sigint(
                    pid,
                    Arc::clone(&suppress_output),
                    Arc::clone(&is_confirming),
                    Arc::clone(&action),
                ),
                _ => unimplemented!(),
            }
        }
    });
}
