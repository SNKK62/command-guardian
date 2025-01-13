use std::os::fd::RawFd;
use std::process::Child;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};

mod signals;
mod stderr;
mod stdin;
mod stdout;

pub(crate) fn handle_io(child: &mut Child, master_stdout: RawFd, master_stderr: RawFd) {
    let action = Arc::new(RwLock::new(signals::SigIntAction::Continue));
    let is_confirming = Arc::new(AtomicBool::new(false));
    let suppress_output = Arc::new(AtomicBool::new(false));

    signals::handle_signals(
        Arc::clone(&suppress_output),
        Arc::clone(&is_confirming),
        Arc::clone(&action),
    );

    let pid = child.id();
    stdin::handle_stdin(
        child,
        pid,
        Arc::clone(&suppress_output),
        Arc::clone(&is_confirming),
        Arc::clone(&action),
    );

    stdout::handle_stdout(master_stdout, Arc::clone(&suppress_output));
    stderr::handle_stderr(master_stderr, Arc::clone(&suppress_output));
}
