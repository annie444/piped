# pipe

A simple command-line tool to capture the standard output, standard error, and
exit code of a command into shell variables.

`pipe` runs a command for you and, instead of streaming the result straight to
your terminal, prints shell variable-assignment statements. When you `eval`
those statements, the command's `stdout`, `stderr`, and exit code become
ordinary variables in your current shell — ready to inspect, branch on, or feed
into the next command.

It understands the syntax of several shells (POSIX `sh`, `bash`, `zsh`, `fish`,
`csh`, and `tcsh`), so the variables it emits are valid wherever you run it.

## Why?

Capturing all three of stdout, stderr, and the exit status of a command in one
shot is awkward in most shells. You end up juggling temporary files, subshells,
and `$?`:

```sh
out=$(mycommand 2>/tmp/err); ec=$?; err=$(cat /tmp/err); rm /tmp/err
```

With `pipe` it's one line:

```sh
eval "$(pipe -- mycommand)"
# $out, $err, and $ec are now set
```

## Installation

From [crates.io](https://crates.io):

```sh
cargo install pipe-cli
```

Or build from source:

```sh
git clone https://github.com/annie444/piped.git
cd piped
cargo install --path .
```

The binary is named `pipe`. A man page (`pipe.1`) is generated at build time and
placed in the target directory.

## Usage

```
pipe [OPTIONS] [-- ] <COMMAND>...
```

`pipe` writes assignment statements to its own standard output. To actually set
the variables in your shell, wrap the call in `eval`:

```sh
eval "$(pipe -- ls -la /nonexistent)"

echo "exit code: $ec"
echo "stderr:    $err"
echo "stdout:    $out"
```

Use `--` to separate `pipe`'s own options from the command you want to run.
This is important when the target command has flags that would otherwise be
interpreted by `pipe`.

### Choosing variable names

By default the captured values are stored in `out`, `err`, and `ec`. You can
rename any of them:

```sh
eval "$(pipe --stdout result --stderr errors --exit-code status -- mycommand)"
echo "$result"
```

### Exporting

By default the variables are set in the current shell only. Pass `-x` /
`--export` to export them into the environment so child processes can see them:

```sh
eval "$(pipe -x -- mycommand)"
```

### Running through a shell

By default the command is executed directly. Pass `-s` / `--sh` to run it
through a shell instead, which enables pipes, globs, redirections, and other
shell features inside the captured command:

```sh
eval "$(pipe -s -- 'cat file.txt | grep error | wc -l')"
echo "$out"
```

The shell used is auto-detected from the parent process, falling back to the
`SHELL` environment variable and then `/bin/sh`. Override it explicitly with
`--shell`:

```sh
eval "$(pipe -s --shell /bin/bash -- 'echo $0')"
```

### Streaming output to the terminal

By default `pipe` captures output silently into the variables. If you also want
to see the output as the command runs, disable capturing:

| Flag                  | Effect                                               |
| --------------------- | ---------------------------------------------------- |
| `-c`, `--no-capture`  | Don't capture; stream both stdout and stderr through |
| `-o`, `--capture-out` | Don't capture stdout; stream it through              |
| `-e`, `--capture-err` | Don't capture stderr; stream it through              |

The variables are still set in every case — these flags only control whether the
output is _also_ echoed to your terminal as it is produced.

> **Note:** When streaming is enabled, the live output is written to **stderr**,
> including the command's own stdout. This is deliberate: `pipe`'s stdout is
> reserved for the assignment statements that `eval` consumes, so echoing live
> output there would corrupt the captured variables. Sending it to stderr keeps
> it visible during `eval "$(pipe ...)"` without polluting that channel.

## Options

| Option                | Default | Description                                                 |
| --------------------- | ------- | ----------------------------------------------------------- |
| `--stdout <NAME>`     | `out`   | Variable name to store the captured standard output         |
| `--stderr <NAME>`     | `err`   | Variable name to store the captured standard error          |
| `--exit-code <NAME>`  | `ec`    | Variable name to store the exit code                        |
| `-x`, `--export`      | off     | Export the variables into the environment                   |
| `-c`, `--no-capture`  | capture | Stream all output to the terminal instead of capturing only |
| `-o`, `--capture-out` | capture | Stream stdout to the terminal                               |
| `-e`, `--capture-err` | capture | Stream stderr to the terminal                               |
| `-s`, `--sh`          | off     | Run the command through a shell                             |
| `--shell <PATH>`      | auto    | Shell to use (for `--sh` and for variable syntax)           |
| `-h`, `--help`        |         | Print help                                                  |
| `-V`, `--version`     |         | Print version                                               |

## Supported shells

`pipe` emits the correct assignment syntax for the detected shell:

| Shell             | Set              | Export               |
| ----------------- | ---------------- | -------------------- |
| `sh`/`bash`/`zsh` | `name=value`     | `export name=value`  |
| `fish`            | `set name value` | `set -gx name value` |
| `csh`/`tcsh`      | `set name value` | `setenv name value`  |

## License

MIT © Annie Ehler
