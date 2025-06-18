use clap::{
    builder::{ArgAction, ArgPredicate}, Parser,
};
use std::{
    io::{BufRead, BufReader}, 
    path::Path, 
    process::{exit, Command, Stdio}, 
    sync::mpsc::{channel, Receiver, Sender},
    thread
};
use shlex::{split, try_quote};

/// A simple CLI application that captures standard output
/// and error of a command
#[derive(Parser, Debug)]
#[clap(name = "pipe", author, version, about, long_about = None)]
pub struct Cli {
    /// The variable to store the standard output in
    #[arg()]
    stdout: String,

    /// The variable to store the standard error in
    #[arg()]
    stderr: String,

    /// The variable to store the exit code in
    /// If not provided, the exit code will not be captured
    #[arg(long = "exit-code")]
    exit_code: Option<String>,

    /// Whether to export the environment variables
    /// or just set them
    #[arg(
        short = 'x', 
        long = "export", 
        default_value_t = false, 
        action = ArgAction::SetTrue
    )]
    export: bool,

    /// Whether to capture the output of the command (default)
    /// or pipe it to the terminal
    #[arg(
        short = 'c', 
        long = "capture", 
        default_value_t = true, 
        action = ArgAction::SetFalse
    )]
    capture: bool,

    /// Whether to capture the output of the command's
    /// standard output (default) or pipe it to the terminal
    /// Defaults to `true` if `capture` is `true`
    #[arg(
        short = 'o',
        long = "capture-out",
        default_value_t = true,
        default_value_if("capture", ArgPredicate::Equals("false".into()), "false"),
        action = ArgAction::SetFalse,
    )]
    capture_out: bool,

    /// Whether to capture the output of the command's
    /// standard error (default) or pipe it to the terminal
    /// Defaults to `true` if `capture` is `true`
    #[arg(
        short = 'e',
        long = "capture-err",
        default_value_t = true,
        default_value_if("capture", ArgPredicate::Equals("false".into()), "false"),
        action = ArgAction::SetFalse,
    )]
    capture_err: bool,

    /// Whether to run the command in a shell
    /// or directly
    #[arg(
        short = 's',
        long = "sh",
        default_value_t = false,
        action = ArgAction::SetTrue
    )]
    sh: bool,

    /// The shell to use if `shell` is true
    /// Defaults to `$SHELL`
    #[arg(
        long, 
        env = "SHELL", 
        default_value = "/bin/sh", 
        action = ArgAction::Set, 
        required_if_eq("sh", "true")
    )]
    shell: String,

    /// The command to run
    /// This may need to be separated with `--` if it conflicts with 
    /// other options or has special flags
    #[arg(action = ArgAction::Append)]
    command: Vec<String>,
}


fn main() {
    let mut cli = Cli::parse();
    let (stdout_sender, stdout_receiver): (Sender<String>, Receiver<String>) = channel();
    let (stderr_sender, stderr_receiver): (Sender<String>, Receiver<String>) = channel();

    cli.command = split(&cli.command.join(" "))
        .unwrap_or_else(|| {
            eprintln!("Failed to parse command arguments");
            exit(1);
        });

    let mut command = if cli.sh {
        let args = cli.command.join(" ");
        let (sh, sh_args) = get_shell(&cli.shell);
        let mut obj = Command::new(sh);
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
    command.stdout(Stdio::piped())
           .stderr(Stdio::piped());
    let (out, err, ec) = if let Ok(mut child) = command.spawn() {
        let mut stdout = BufReader::new(child.stdout.take().expect("Failed to capture stdout"));
        let mut stderr = BufReader::new(child.stderr.take().expect("Failed to capture stderr"));
        let stdout_reader = thread::spawn(move || {
            let mut out = String::new();
            while let Ok(len) = stdout.read_line(&mut out) {
                if len == 0 {
                    break; // EOF
                }
                if !cli.capture || !cli.capture_out {
                    if !out.is_empty() {
                        eprint!("{}", out);
                    }
                }
                stdout_sender.send(out.clone()).expect("Failed to send stdout");
                out.clear();
            }
        });
        let stderr_reader = thread::spawn(move || {
            let mut err = String::new();
            while let Ok(len) = stderr.read_line(&mut err) {
                if len == 0 {
                    break; // EOF
                }
                if !cli.capture || !cli.capture_err {
                    if !err.is_empty() {
                        eprint!("{}", err);
                    }
                }
                stderr_sender.send(err.clone()).expect("Failed to send stderr");
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
                eprintln!("Failed to wait for child process: {}", e);
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

    let var = set_var(&cli.shell, cli.export, &cli.stdout, &out);
    println!("{};", var);
    let var = set_var(&cli.shell, cli.export, &cli.stderr, &err);
    println!("{};", var);
    if let Some(ref exit_code) = cli.exit_code {
        let var = set_var(&cli.shell, cli.export, exit_code, &ec.to_string());
        println!("{};", var);
    }
}

fn get_shell<'a>(sh: &'a str) -> (&'a str, Vec<&'static str>) {
    let shell = Path::new(sh);
    match shell.file_name().unwrap().to_str().unwrap() {
        "sh" | "zsh" => (sh, vec!["-c"]),
        "nu" | "fish" | "bash" => (sh, vec!["-l", "-c"]),
        "csh" | "tcsh" => (sh, vec!["-d", "-e", "-c"]),
        _ => ("/bin/sh", vec!["-c"]),
    }
}

fn set_var(shell: &str, export: bool, name: &str, value: &str) -> String {
    let value = match try_quote(value) {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Failed to quote value: {}", value);
            exit(1);
        }
    };
    let shell = Path::new(shell);
    match shell.file_name().unwrap().to_str().unwrap() {
        "sh" | "zsh" | "bash" => {
            if export {
                format!("export {}={}", name, value)
            } else {
                format!("{}={}", name, value)
            }
        },
        "fish" => {
            if export {
                format!("set -gx {} {}", name, value)
            } else {
                format!("set {} {}", name, value)
            }
        }
        "csh" | "tcsh" => {
            if export {
                format!("setenv {} {}", name, value)
            } else {
                format!("set {} {}", name, value)
            }
        }
        _ => String::new(),
    }
}
