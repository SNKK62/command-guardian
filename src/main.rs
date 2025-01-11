use signal_hook::consts::SIGINT;
use signal_hook::iterator::Signals;
use std::io;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

#[allow(clippy::needless_borrows_for_generic_args)]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <command>", args[0]);
        std::process::exit(1);
    }

    let command = &args[1];
    let command_args = &args[2..];

    let is_running = Arc::new(AtomicBool::new(true));
    let suppress_output = Arc::new(AtomicBool::new(false));
    let child_process = Arc::new(Mutex::new(spawn_command(
        command,
        command_args,
        Arc::clone(&suppress_output),
    )));

    let is_running_clone = Arc::clone(&is_running);
    let child_process_clone = Arc::clone(&child_process);
    thread::spawn(move || {
        let mut signals = Signals::new(&[SIGINT]).expect("Failed to set up signal handling");
        for _ in signals.forever() {
            suppress_output.store(true, Ordering::SeqCst);
            print!("\nCtrl-C detected. Do you want to terminate the command? (Y/[n]):");
            io::stdout().flush().expect("Failed to flush stdout");
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read input");
            if input.trim().eq("Y") {
                println!("Terminating command...");
                if let Ok(mut child) = child_process_clone.lock() {
                    if child.as_mut().is_some() {
                        child
                            .as_mut()
                            .unwrap()
                            .kill()
                            .expect("Failed to kill child process");
                    }
                }
                is_running_clone.store(false, Ordering::SeqCst);
                println!("Program terminated.");
                break;
            } else {
                println!("Continuing command...");
            }
            suppress_output.store(false, Ordering::SeqCst);
        }
    });

    while is_running.load(Ordering::SeqCst) {
        if let Ok(mut child) = child_process.lock() {
            if let Some(ref mut c) = child.as_mut() {
                if let Ok(Some(_)) = c.try_wait() {
                    println!("Command finished.");
                    break;
                }
            }
        }
        thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn spawn_command(
    command: &str,
    args: &[String],
    suppress_output: Arc<AtomicBool>,
) -> Option<Child> {
    let mut cmd = Command::new(command);
    let child = cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
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

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let stderr = child.stderr.take().expect("Failed to capture stderr");

    let suppress_stdout = Arc::clone(&suppress_output);
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if !suppress_stdout.load(Ordering::SeqCst) && line.is_ok() {
                println!("{}", line.unwrap());
            }
        }
    });

    let suppress_stderr = Arc::clone(&suppress_output);
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if !suppress_stderr.load(Ordering::SeqCst) && line.is_ok() {
                eprintln!("{}", line.unwrap());
            }
        }
    });

    Some(child)
}
