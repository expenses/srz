use ignore::Walk;
use std::collections::{HashMap, hash_map::Entry};
use std::path::{PathBuf, Path};
use std::fs::{read, File};
use clap::{App, Arg, SubCommand, ArgMatches};
use read_input::prelude::*;
use std::io::Write;
use ansi_term::{Style, Colour};

type Descriptions = HashMap<PathBuf, String>;

struct Sunrise {
    descriptions: Descriptions,
    path: PathBuf,
    root_path: PathBuf
}

impl Sunrise {
    fn locate(start: &Path) -> Result<Self, Error> {
        let mut current_dir = start.to_path_buf();
        let mut not_at_root = true;

        while not_at_root {
            let path = current_dir.join(".sunrise");

            if path.exists() {
                println!("{}", Style::new().bold().paint(
                    format!("Using {}", path.display())
                ));

                let descriptions = read(&path)
                    .map_err(Error::Io)
                    .and_then(|file| toml::from_slice(&file)
                    .map_err(Error::TomlDe))?;

                return Ok(Sunrise {
                    path,
                    descriptions,
                    root_path: current_dir
                });
            }

            not_at_root = current_dir.pop();
        }

        return Err(Error::Static(".sunrise not found"));
    }

    fn save(&self) -> Result<(), Error> {
        let mut file = File::create(&self.path).map_err(Error::Io)?;
        let buffer = toml::to_vec(&self.descriptions).map_err(Error::TomlSer)?;
        file.write_all(&buffer).map_err(Error::Io)
    }

    fn relative(&self, path: &Path) -> PathBuf {
        pathdiff::diff_paths(path, &self.root_path).unwrap()
    }

    fn add_description(&mut self, filename: &Path) -> Result<bool, Error> {
        let string = read_input::InputBuilder::<String>::new()
            .msg(format!(
                "{}: Add description or press enter to ignore: ",
                Style::new().bold().paint(filename.display().to_string())
            ))
            .get();

        if !string.is_empty() {
            self.descriptions.insert(filename.to_path_buf(), string);
            self.save()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn files<'a>(&'a self, subdirectory: &'a Path) -> impl Iterator<Item=PathBuf> + 'a {
        Walk::new(subdirectory)
            .filter_map(|entry| entry.ok())
            .map(move |entry| self.relative(entry.path()))
            .filter(|path| !path.as_os_str().is_empty())
    }

    fn clean(&mut self) -> Result<bool, Error> {
        let files: Vec<_> = self.descriptions.keys().map(|key| key.clone()).collect();
        let mut altered = false;

        for file in files {
            if !self.path.join(&file).exists() {
                let input = read_input::InputBuilder::<Decision>::new()
                    .msg(&format!("{} no longer exists. Would you like to remove it? [Y/n]: ", file.display()))
                    .default(Decision::Yes)
                    .get();

                if let Decision::Yes = input {
                    self.descriptions.remove(&file);
                    altered = true;
                    self.save()?;
                }
            }
        }

        Ok(altered)
    }
}

mod subcommands;

#[derive(Debug)]
enum Decision {
    Yes,
    No
}

impl std::str::FromStr for Decision {
    type Err = ();

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string.to_lowercase().as_str() {
            "y" | "yes" | "yes please" => Ok(Decision::Yes),
            "n" | "no" | "no thanks" => Ok(Decision::No),
            _ => Err(())
        }
    }
}

fn main() {
    let matches = App::new("Sunrise")
        .arg(Arg::with_name("DIRECTORY").takes_value(true))
        .subcommand(SubCommand::with_name("init")
            .about("Initialise sunrise in a directory")
            .arg(Arg::with_name("DIRECTORY").takes_value(true))
        )
        .subcommand(SubCommand::with_name("interactive")
            .about("Interactive mode")
            .arg(Arg::with_name("DIRECTORY")
                .takes_value(true)
            )
        )
        .subcommand(SubCommand::with_name("edit")
            .about("Edit or add a description for an file/directory")
            .arg(Arg::with_name("FILE")
                .required(true)
                .takes_value(true)
            )
        )
        .get_matches();

    println!("{:?}", matches);

    if let Err(error) = run(matches) {
        println!("{}: {}", Style::new().bold().fg(Colour::Red).paint("Error"), error);
    }
}

fn run(matches: ArgMatches) -> Result<(), Error> {
    if let Some(matches) = matches.subcommand_matches("init") {
        subcommands::init(
            directory_or_current(matches.value_of("DIRECTORY"))?
        );
    } else if let Some(matches) = matches.subcommand_matches("interactive") {
        subcommands::interactive(
            directory_or_current(matches.value_of("DIRECTORY"))?
        ).unwrap();
    } else if let Some(matches) = matches.subcommand_matches("edit") {
        let filename = matches.value_of("FILE").map(Path::new).unwrap();
        subcommands::edit(&filename)?;
    } else {
        subcommands::print(
            directory_or_current(matches.value_of("DIRECTORY"))?
        );
    }

    Ok(())
}

fn write_descriptions(descriptions: &Descriptions, path: &Path) -> Option<()> {
    let mut file = File::create(&path.join(".sunrise")).ok()?;
    let buffer = toml::to_vec(&descriptions).ok()?;
    file.write_all(&buffer).ok()
}

fn entry_description(path: &Path) -> Option<String> {
    let string = read_input::InputBuilder::<String>::new()
        .msg(format!(
            "{}: Add description or press enter to ignore: ",
            Style::new().bold().paint(path.display().to_string())
        ))
        .get();

    if !string.is_empty() {
        Some(string)
    } else {
        None
    }
}

fn sunrise_dir(directory: &Path) -> Result<PathBuf, Error> {
    let mut current_dir = directory.to_path_buf();
    let mut not_at_root = true;

    while not_at_root {
        let sunrise_path = current_dir.join(".sunrise");

        if sunrise_path.exists() {
            println!("{}", Style::new().bold().paint(
                format!("Using {}", sunrise_path.display())
            ));
            return Ok(current_dir);
        }

        not_at_root = current_dir.pop();
    }

    return Err(Error::Static(".sunrise not found"));
}

fn directory_or_current(dir: Option<&str>) -> Result<PathBuf, Error> {
    dir
        .map(|dir| Ok(PathBuf::from(dir)))
        .unwrap_or_else(|| std::env::current_dir().map_err(Error::Io))
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    TomlSer(toml::ser::Error),
    TomlDe(toml::de::Error),
    Static(&'static str),
    Dynamic(String)
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => e.fmt(f),
            Error::TomlSer(e) => e.fmt(f),
            Error::TomlDe(e) => e.fmt(f),
            Error::Static(e) => e.fmt(f),
            Error::Dynamic(e) => e.fmt(f)

        }
    }
}
