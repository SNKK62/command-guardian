use std::os::fd::RawFd;
use std::process::Child;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};

mod input;
mod output;
mod signals;

pub(crate) enum OutputType {
    Stdout,
    Stderr,
}

pub(crate) fn handle_io(child: &mut Child, master_stdout: RawFd, master_stderr: RawFd) {
    let action = Arc::new(RwLock::new(signals::SigIntAction::Continue));
    let is_confirming = Arc::new(AtomicBool::new(false));
    let suppress_output = Arc::new(AtomicBool::new(false));

    let pid = child.id();

    signals::handle_signals(
        pid,
        Arc::clone(&suppress_output),
        Arc::clone(&is_confirming),
        Arc::clone(&action),
    );

    input::handle_stdin(
        child,
        Arc::clone(&suppress_output),
        Arc::clone(&is_confirming),
        Arc::clone(&action),
    );

    output::handle_output(
        master_stdout,
        Arc::clone(&suppress_output),
        OutputType::Stdout,
    );
    output::handle_output(
        master_stderr,
        Arc::clone(&suppress_output),
        OutputType::Stderr,
    );
}
