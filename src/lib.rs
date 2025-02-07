use nix::pty::{openpty, OpenptyResult, Winsize};
use std::os::fd::{FromRawFd, RawFd};
use std::os::unix::io::IntoRawFd;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};

mod io;
use io::handle_io;

fn get_terminal_size() -> termsize::Size {
    match termsize::get() {
        Some(size) => size,
        None => termsize::Size { rows: 24, cols: 80 },
    }
}

fn create_pty(terminal_size: &termsize::Size) -> (RawFd, RawFd) {
    let OpenptyResult { master, slave } = openpty(
        Some(&Winsize {
            ws_row: terminal_size.rows,
            ws_col: terminal_size.cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        }),
        None,
    )
    .expect("\x1b[31mFailed to create PTY\x1b[0m");
    (master.into_raw_fd(), slave.into_raw_fd())
}

pub fn spawn_command(command: &str, args: &[String]) -> Option<Child> {
    let term_size = get_terminal_size();

    let (master_stdout, slave_stdout) = create_pty(&term_size);
    let (master_stderr, slave_stderr) = create_pty(&term_size);

    let mut cmd = Command::new(command);
    let child = cmd
        .args(args)
        .stdin(Stdio::piped())
        .stdout(unsafe { Stdio::from_raw_fd(slave_stdout) })
        .stderr(unsafe { Stdio::from_raw_fd(slave_stderr) });

    unsafe {
        child.pre_exec(|| {
            // create a new session for the child process
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = match child.spawn() {
        Ok(child) => {
            println!(
                "\x1b[32mInvoked child process successfully (PID: \x1b[1;32m{}\x1b[32m)\x1b[0m",
                child.id()
            );
            child
        }
        Err(e) => {
            println!("\x1b[31mFailed invoke child process: {}\x1b[0m", e);
            std::process::exit(1);
        }
    };

    handle_io(&mut child, master_stdout, master_stderr);

    Some(child)
}
