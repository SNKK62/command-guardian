use nix::pty::{openpty, OpenptyResult, Winsize};
use signal_hook::consts::SIGINT;
use signal_hook::iterator::Signals;
use std::io;
use std::io::{BufReader, Read, Write};
use std::os::fd::FromRawFd;
use std::os::unix::io::{AsRawFd, IntoRawFd};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

enum Action {
    Continue,
    Terminate,
    Invalid,
}

#[allow(clippy::needless_borrows_for_generic_args)]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <command>", args[0]);
        std::process::exit(1);
    }

    let command = &args[1];
    let command_args = &args[2..];

    let suppress_output = Arc::new(AtomicBool::new(false));
    let is_confirming = Arc::new(AtomicBool::new(false));
    let action = Arc::new(RwLock::new(Action::Continue));

    let child_process = Arc::new(Mutex::new(spawn_command(
        command,
        command_args,
        Arc::clone(&suppress_output),
        Arc::clone(&is_confirming),
        Arc::clone(&action),
    )));

    thread::spawn(move || {
        let mut signals = Signals::new(&[SIGINT]).expect("Failed to set up signal handling");
        for _ in signals.forever() {
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
                    Action::Continue => {
                        suppress_output.store(false, Ordering::SeqCst);
                        break;
                    }
                    Action::Terminate => {
                        println!("Program terminated.");
                        break;
                    }
                    Action::Invalid => {
                        is_confirming.store(true, Ordering::SeqCst);
                        continue;
                    }
                }
            }
        }
    });

    loop {
        if let Ok(mut child) = child_process.lock() {
            if let Some(ref mut c) = child.as_mut() {
                if let Ok(Some(_)) = c.try_wait() {
                    println!("Command finished.");
                    break;
                }
            }
        }
        thread::sleep(std::time::Duration::from_millis(300));
    }
}

fn spawn_command(
    command: &str,
    args: &[String],
    suppress_output: Arc<AtomicBool>,
    is_confirming: Arc<AtomicBool>,
    action: Arc<RwLock<Action>>,
) -> Option<Child> {
    let term_size = match termsize::get() {
        Some(size) => size,
        None => termsize::Size { rows: 24, cols: 80 },
    };

    // create a new PTY for stdout
    let OpenptyResult {
        master: master_stdout,
        slave: slave_stdout,
    } = openpty(
        Some(&Winsize {
            ws_row: term_size.rows as u16,
            ws_col: term_size.cols as u16,
            ws_xpixel: 0,
            ws_ypixel: 0,
        }),
        None,
    )
    .expect("Failed to create PTY");

    // create a new PTY for stderr
    let OpenptyResult {
        master: master_stderr,
        slave: slave_stderr,
    } = openpty(
        Some(&Winsize {
            ws_row: term_size.rows as u16,
            ws_col: term_size.cols as u16,
            ws_xpixel: 0,
            ws_ypixel: 0,
        }),
        None,
    )
    .expect("Failed to create PTY");

    let slave_stdout_fd = slave_stdout.as_raw_fd();
    let slave_stderr_fd = slave_stderr.as_raw_fd();

    let mut cmd = Command::new(command);
    let child = cmd
        .args(args)
        .stdin(Stdio::piped())
        .stdout(unsafe { Stdio::from_raw_fd(slave_stdout_fd) })
        .stderr(unsafe { Stdio::from_raw_fd(slave_stderr_fd) });

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
            println!("Invoked child process successfully (PID: {})", child.id());
            child
        }
        Err(e) => {
            println!("Failed invoke child proces: {}", e);
            std::process::exit(1);
        }
    };

    let suppress_stdin = Arc::clone(&suppress_output);
    let pid = child.id();
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
                println!("input: {}", input);
                #[allow(clippy::collapsible_else_if)]
                if input.trim().eq("Y") {
                    println!("Terminating command...");
                    unsafe {
                        libc::kill(pid as i32, SIGINT);
                    }
                    suppress_stdin.store(false, Ordering::SeqCst);
                    *action = Action::Terminate;
                    break;
                } else if input.trim().eq_ignore_ascii_case("n") || input.trim().is_empty() {
                    println!("Continuing command...");
                    suppress_stdin.store(false, Ordering::SeqCst);
                    *action = Action::Continue;
                } else {
                    println!("Invalid input. You must enter 'Y', 'n', 'N' or just press ENTER.");
                    *action = Action::Invalid;
                }
                is_confirming.store(false, Ordering::SeqCst);
            }
        }
    });

    let suppress_stdout = Arc::clone(&suppress_output);
    let mut stdout_reader =
        BufReader::new(unsafe { std::fs::File::from_raw_fd(master_stdout.into_raw_fd()) });
    thread::spawn(move || {
        let mut buffer = [0; 1024];
        while let Ok(n) = stdout_reader.read(&mut buffer) {
            if n == 0 {
                break;
            }
            if !suppress_stdout.load(Ordering::SeqCst) {
                let output = String::from_utf8_lossy(&buffer[..n]);
                print!("{}", output);
                std::io::stdout().flush().unwrap();
            }
        }
    });

    let suppress_stderr = Arc::clone(&suppress_output);
    let mut stderr_reader =
        BufReader::new(unsafe { std::fs::File::from_raw_fd(master_stderr.into_raw_fd()) });
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

    Some(child)
}
