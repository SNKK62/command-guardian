use std::sync::{Arc, Mutex};
use std::thread;

use command_guardian::spawn_command;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("\x1b[31mUsage: {} <command>\x1b[0m", args[0]);
        std::process::exit(1);
    }

    let command = &args[1];
    let command_args = &args[2..];

    let child_process = Arc::new(Mutex::new(spawn_command(command, command_args)));

    loop {
        if let Ok(mut child) = child_process.lock() {
            if let Some(ref mut c) = child.as_mut() {
                if let Ok(Some(_)) = c.try_wait() {
                    break;
                }
            }
        }
        thread::sleep(std::time::Duration::from_millis(300));
    }
}
