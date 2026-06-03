use clap::{
    Parser,
    builder::{ArgAction, ArgPredicate},
};

const STDOUT_VAR: &str = "out";
const STDERR_VAR: &str = "err";
const EC_VAR: &str = "ec";

/// A simple CLI application that captures standard output
/// and error of a command
#[derive(Parser, Debug)]
#[clap(name = "pipe", author, version, about, long_about = None)]
pub struct Pipe {
    /// The shell variable to store the standard output in
    #[arg(
        long = "stdout",
        action = ArgAction::Set,
        default_value = STDOUT_VAR,
    )]
    pub stdout: String,

    /// The shell variable to store the standard error in
    #[arg(
        long = "stderr",
        action = ArgAction::Set,
        default_value = STDERR_VAR,
    )]
    pub stderr: String,

    /// The shell variable to store the exit code in
    #[arg(
        long = "exit-code",
        action = ArgAction::Set,
        default_value = EC_VAR,
    )]
    pub exit_code: String,

    /// Whether to export the environment variables
    /// or just set them
    #[arg(
        short = 'x',
        long = "export",
        default_value_t = false,
        action = ArgAction::SetTrue
    )]
    pub export: bool,

    /// Whether to capture the output of the command (default)
    /// or pipe it to the terminal
    #[arg(
        short = 'c',
        long = "capture",
        default_value_t = true,
        action = ArgAction::SetFalse
    )]
    pub capture: bool,

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
    pub capture_out: bool,

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
    pub capture_err: bool,

    /// Whether to run the command in a shell
    /// or directly
    #[arg(
        short = 's',
        long = "sh",
        default_value_t = false,
        action = ArgAction::SetTrue
    )]
    pub sh: bool,

    /// The shell to use if `shell` is true
    /// Also used to determine the syntax for setting variables
    /// If not provided, the shell of the parent process is used.
    /// If the parent process shell cannot be determined, it will
    /// fallback to the `SHELL` environment variable or `/bin/sh`.
    #[arg(
        long,
        action = ArgAction::Set,
    )]
    pub shell: Option<String>,

    /// The command to run
    /// This may need to be separated with `--` if it conflicts with
    /// other options or has special flags
    #[arg(action = ArgAction::Append)]
    pub command: Vec<String>,
}
