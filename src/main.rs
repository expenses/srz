use ignore::Walk;
use std::collections::BTreeMap;
use std::path::{PathBuf, Path};
use std::fs::{read, File};
use read_input::prelude::*;
use std::io::Write;
use ansi_term::{Style, Colour};
use structopt::StructOpt;

// Paths should be ordered
type Descriptions = BTreeMap<PathBuf, String>;

#[derive(Debug)]
struct Sunrise {
    descriptions: Descriptions,
    file_path: PathBuf,
    repo_path: PathBuf
}

impl Sunrise {
    fn locate(start: &Path, verbose: bool) -> Result<Self, Error> {
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

                let sunrise = Sunrise {
                    file_path: path,
                    descriptions,
                    repo_path: current_dir.canonicalize().map_err(Error::Io)?
                };

                if verbose {
                    println!("Sunrise.file_path: {}", sunrise.file_path.display());
                    println!("Sunrise.repo_path: {}", sunrise.repo_path.display());
                    println!("#Sunrise.descriptions: {}", sunrise.descriptions.len());
                    println!("-----")
                }

                return Ok(sunrise);
            }

            not_at_root = current_dir.pop();
        }

        Err(Error::Static(".sunrise not found"))
    }

    fn save(&self) -> Result<(), Error> {
        let mut file = File::create(&self.file_path).map_err(Error::Io)?;
        let buffer = toml::to_vec(&self.descriptions).map_err(Error::TomlSer)?;
        file.write_all(&buffer).map_err(Error::Io)
    }

    fn relative(&self, path: &Path) -> PathBuf {
        pathdiff::diff_paths(&path.canonicalize().unwrap(), &self.repo_path).unwrap()
    }

    fn set_description(&mut self, filename: &Path, string: String) -> Result<(), Error> {
        self.descriptions.insert(filename.to_path_buf(), string);
        self.save()
    }

    fn add_description(&mut self, filename: &Path, text: &str) -> Result<bool, Error> {
        let string = read_input::InputBuilder::<String>::new()
            .msg(format!(
                "{}: {}: ",
                Style::new().bold().paint(filename.display().to_string()),
                text
            ))
            .get();

        if !string.is_empty() {
            self.set_description(filename, string)?;
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

    fn descripted(&self) -> Vec<PathBuf> {
        self.descriptions.keys().map(PathBuf::clone).collect()
    }

    fn remove(&mut self, file: &PathBuf) -> Result<(), Error> {
        self.descriptions.remove(file);
        self.save()
    }

    fn clean(&mut self) -> Result<bool, Error> {
        let mut altered = false;

        for file in self.descripted() {
            if !self.repo_path.join(&file).exists() {
                let input = read_input::InputBuilder::<Decision>::new()
                    .msg(&format!("{} no longer exists. Would you like to remove it? [Y/n]: ", file.display()))
                    .default(Decision::Yes)
                    .get();

                if let Decision::Yes = input {
                    self.remove(&file)?;
                    altered = true;
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

#[derive(StructOpt, Debug)]
enum Subcommand {
    Interactive {
        directory: Option<PathBuf>,
    },
    Init {
        directory: Option<PathBuf>,
    },
    Edit {
        files: Vec<PathBuf>,
    },
    Review {
        directory: Option<PathBuf>,
    }
}

#[derive(StructOpt, Debug)]
struct Opt {
    #[structopt(subcommand)]
    subcommand: Option<Subcommand>,
    #[structopt(short, long)]
    inline: bool,
    #[structopt(short, long)]
    verbose: bool,
    directory: Option<PathBuf>
}

fn main() {
    let opt = Opt::from_args();
    if opt.verbose {
        println!("{:?}", opt);
    }

    if let Err(error) = run(opt) {
        println!("{}: {}", Style::new().bold().fg(Colour::Red).paint("Error"), error);
    }
}

fn run(opt: Opt) -> Result<(), Error> {
    match opt.subcommand {
        None => subcommands::print(directory_or_current(opt.directory)?, opt.verbose, opt.inline),
        Some(Subcommand::Init {directory}) => subcommands::init(directory_or_current(directory)?),
        Some(Subcommand::Interactive {directory}) => subcommands::interactive(
            directory_or_current(directory)?, opt.verbose
        ),
        Some(Subcommand::Review {directory}) => subcommands::review(directory_or_current(directory)?, opt.verbose),
        Some(Subcommand::Edit {files}) => subcommands::edit(files, opt.verbose)
    }
}

fn directory_or_current(dir: Option<PathBuf>) -> Result<PathBuf, Error> {
    dir.map(Result::Ok).unwrap_or_else(|| std::env::current_dir().map_err(Error::Io))
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
