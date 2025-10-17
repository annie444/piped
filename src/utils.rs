use shlex::try_quote;
use std::{env, path::Path, process, process::exit};
use sysinfo::{Pid, System};

pub fn shell_or_sh() -> String {
    env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
}

pub fn get_shell(sh: Option<String>) -> (String, Vec<&'static str>) {
    let shell = sh.unwrap_or_else(|| {
        let system = System::new_all();
        if let Some(self_proc) = system.process(Pid::from(process::id() as usize)) {
            if let Some(ppid) = self_proc.parent() {
                if let Some(parent) = system.process(ppid) {
                    if let Some(shell_path) = parent.exe() {
                        shell_path.to_string_lossy().into_owned()
                    } else {
                        shell_or_sh()
                    }
                } else {
                    shell_or_sh()
                }
            } else {
                shell_or_sh()
            }
        } else {
            shell_or_sh()
        }
    });
    let shell_path = Path::new(&shell);
    match shell_path.file_name().unwrap().to_str().unwrap() {
        "sh" | "zsh" => (shell, vec!["-c"]),
        "nu" | "fish" | "bash" => (shell, vec!["-l", "-c"]),
        "csh" | "tcsh" => (shell, vec!["-d", "-e", "-c"]),
        _ => ("/bin/sh".to_string(), vec!["-c"]),
    }
}

pub fn set_var(shell: &str, export: bool, name: &str, value: &str) -> String {
    let value = match try_quote(value) {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Failed to quote value: {value}");
            exit(1);
        }
    };
    let shell = Path::new(shell);
    match shell.file_name().unwrap().to_str().unwrap() {
        "sh" | "zsh" | "bash" => {
            if export {
                format!("export {name}={value}")
            } else {
                format!("{name}={value}")
            }
        }
        "fish" => {
            if export {
                format!("set -gx {name} {value}")
            } else {
                format!("set {name} {value}")
            }
        }
        "csh" | "tcsh" => {
            if export {
                format!("setenv {name} {value}")
            } else {
                format!("set {name} {value}")
            }
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_set_var_bash() {
        let sh = "/bin/bash";
        let export = true;
        let name = "TEST_VAR";
        let value = "test_value";
        let result = set_var(sh, export, name, value);
        assert_eq!(result, "export TEST_VAR=test_value");
        let export = false;
        let result = set_var(sh, export, name, value);
        assert_eq!(result, "TEST_VAR=test_value");
    }

    #[test]
    fn test_set_var_fish() {
        let sh = "/usr/bin/fish";
        let export = true;
        let name = "TEST_VAR";
        let value = "test_value";
        let result = set_var(sh, export, name, value);
        assert_eq!(result, "set -gx TEST_VAR test_value");
        let export = false;
        let result = set_var(sh, export, name, value);
        assert_eq!(result, "set TEST_VAR test_value");
    }

    #[test]
    fn test_set_var_csh() {
        let sh = "/bin/csh";
        let export = true;
        let name = "TEST_VAR";
        let value = "test_value";
        let result = set_var(sh, export, name, value);
        assert_eq!(result, "setenv TEST_VAR test_value");
        let export = false;
        let result = set_var(sh, export, name, value);
        assert_eq!(result, "set TEST_VAR test_value");
    }

    #[test]
    fn test_get_shell_default() {
        let (shell, args) = get_shell(None);
        unsafe {
            std::env::remove_var("SHELL");
        }
        let expected_shell = shell_or_sh();
        assert_eq!(shell, expected_shell);
        let expected_args = match Path::new(&expected_shell)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
        {
            "sh" | "zsh" => vec!["-c"],
            "nu" | "fish" | "bash" => vec!["-l", "-c"],
            "csh" | "tcsh" => vec!["-d", "-e", "-c"],
            _ => vec!["-c"],
        };
        assert_eq!(args, expected_args);
    }

    #[test]
    fn test_get_shell_specified() {
        let (shell, args) = get_shell(Some("/bin/bash".to_string()));
        assert_eq!(shell, "/bin/bash");
        assert_eq!(args, vec!["-l", "-c"]);
    }
}
