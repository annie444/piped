use std::env;
use std::io::ErrorKind;
use std::path::PathBuf;

use clap::CommandFactory;
use clap_mangen::Man;

std::include!("src/cli.rs");

fn main() -> std::io::Result<()> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").ok_or(ErrorKind::NotFound)?);
    let profile = env::var("PROFILE").map_err(|_| std::io::Error::from(ErrorKind::NotFound))?;
    let profile_dir = match profile.as_str() {
        "release" | "debug" => out_dir
            .ancestors()
            .nth(3)
            .ok_or(ErrorKind::NotFound)?
            .to_path_buf(),
        _ => {
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                format!("Unknown profile: {profile}"),
            ));
        }
    };

    let man = Man::new(Pipe::command());

    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer)?;

    std::fs::write(profile_dir.join("pipe.1"), buffer)?;

    Ok(())
}
