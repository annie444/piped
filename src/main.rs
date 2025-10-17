use clap::Parser;
use shlex::split;
use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio, exit},
    sync::mpsc::{Receiver, Sender, channel},
    thread,
};

mod cli;
mod utils;

use cli::Pipe;

fn main() {
    let mut cli = Pipe::parse();
    let (stdout_sender, stdout_receiver): (Sender<String>, Receiver<String>) = channel();
    let (stderr_sender, stderr_receiver): (Sender<String>, Receiver<String>) = channel();

    cli.command = split(&cli.command.join(" ")).unwrap_or_else(|| {
        eprintln!("Failed to parse command arguments");
        exit(1);
    });

    let (sh, sh_args) = utils::get_shell(cli.shell);

    let mut command = if cli.sh {
        let args = cli.command.join(" ");
        let mut obj = Command::new(&sh);
        obj.args(sh_args);
        obj.arg(args);
        obj
    } else {
        let name = &cli.command[0];
        let args = Vec::from(&cli.command[1..]);
        let mut obj = Command::new(name);
        obj.args(args);
        obj
    };

    command.stdout(Stdio::piped()).stderr(Stdio::piped());

    let (out, err, ec) = if let Ok(mut child) = command.spawn() {
        let mut stdout = BufReader::new(child.stdout.take().expect("Failed to capture stdout"));
        let mut stderr = BufReader::new(child.stderr.take().expect("Failed to capture stderr"));
        let stdout_reader = thread::spawn(move || {
            let mut out = String::new();
            while let Ok(len) = stdout.read_line(&mut out) {
                if len == 0 {
                    break; // EOF
                }
                if (!cli.capture || !cli.capture_out) && !out.is_empty() {
                    eprint!("{out}");
                }
                stdout_sender
                    .send(out.clone())
                    .expect("Failed to send stdout");
                out.clear();
            }
        });
        let stderr_reader = thread::spawn(move || {
            let mut err = String::new();
            while let Ok(len) = stderr.read_line(&mut err) {
                if len == 0 {
                    break; // EOF
                }
                if (!cli.capture || !cli.capture_err) && !err.is_empty() {
                    eprint!("{err}");
                }
                stderr_sender
                    .send(err.clone())
                    .expect("Failed to send stderr");
                err.clear();
            }
        });
        let stdout_writer = thread::spawn(move || {
            let mut out = String::new();
            while let Ok(msg) = stdout_receiver.recv() {
                if msg.is_empty() {
                    continue; // Skip empty messages
                }
                out.push_str(&msg);
            }
            out
        });
        let stderr_writer = thread::spawn(move || {
            let mut err = String::new();
            while let Ok(msg) = stderr_receiver.recv() {
                if msg.is_empty() {
                    continue; // Skip empty messages
                }
                err.push_str(&msg);
            }
            err
        });
        let ec = match child.wait() {
            Ok(status) => status.code().unwrap_or(1),
            Err(e) => {
                eprintln!("Failed to wait for child process: {e}");
                1
            }
        };
        let _ = stdout_reader.join();
        let _ = stderr_reader.join();
        let out = match stdout_writer.join() {
            Ok(output) => output,
            Err(_) => {
                eprintln!("Failed to join stdout writer thread");
                String::new()
            }
        };
        let err = match stderr_writer.join() {
            Ok(error) => error,
            Err(_) => {
                eprintln!("Failed to join stderr writer thread");
                String::new()
            }
        };
        (out, err, ec)
    } else {
        eprintln!("Failed to spawn command");
        exit(1);
    };

    let var = utils::set_var(&sh, cli.export, &cli.stdout, &out);
    println!("{var};");
    let var = utils::set_var(&sh, cli.export, &cli.stderr, &err);
    println!("{var};");
    if let Some(ref exit_code) = cli.exit_code {
        let var = utils::set_var(&sh, cli.export, exit_code, &ec.to_string());
        println!("{var};");
    }
}
