use super::*;

pub fn init(mut path: PathBuf) -> Result<(), Error> {
    path.push(".sunrise");

    if path.exists() {
        println!("{} already exists.", path.display());
    } else {
        File::create(&path).map_err(Error::Io)?;
        println!("Initialised {}", path.display());
    }

    Ok(())
}

pub fn interactive(directory: PathBuf, verbose: bool) -> Result<(), Error> {
    let mut sunrise = Sunrise::locate(&directory, verbose)?;

    let mut changed = false;

    let paths: Vec<_> = sunrise.files(&directory).collect();

    for path in paths {
        if !sunrise.descriptions.contains_key(&path.to_path_buf()) {
            let added = sunrise.add_description(&path, "Add description or press enter to ignore");
            changed = changed || added?;
        }
    }

    changed = changed || sunrise.clean()?;

    if !changed {
        println!("Nothing to do :^)");
    }

    Ok(())
}

pub fn print(directory: PathBuf, verbose: bool, inline: bool) -> Result<(), Error> {
    let sunrise = Sunrise::locate(&directory, verbose)?;

    for file in sunrise.files(&directory) {
        let indents = file.iter().count() - 1;

        let indent_string: String = std::iter::repeat(" ").take(indents * 2).collect();

        let indented_file = format!("{}{}", indent_string, file.display());

        let mut printed = false;

        if let Some(desc) = sunrise.descriptions.get(&file) {
            let indent = if inline {&indented_file} else {&indent_string};

            println!("{} // {}", indent, Style::new().bold().paint(desc));
            printed = true;
        }

        if !(inline && printed) {
            println!("{}", indented_file);
        }
    }

    Ok(())
}

pub fn edit(files: Vec<PathBuf>, verbose: bool) -> Result<(), Error> {
    let filename = &files[0];

    let mut sunrise = Sunrise::locate(filename, verbose)?;

    let full_name = std::env::current_dir().map_err(Error::Io)?.join(filename);

    let relative = sunrise.relative(&full_name);

    if !full_name.exists()  {
        return Err(Error::Dynamic(
            format!("{} does not exist.", filename.display())
        ))
    }

    if let Some(description) = sunrise.descriptions.get(&relative) {
        println!("Previous description: {}", description);
    }

    sunrise.add_description(&relative, "Enter a new description")?;

    Ok(())
}

pub fn review(directory: PathBuf, verbose: bool) -> Result<(), Error> {
    let mut sunrise = Sunrise::locate(&directory, verbose)?;

    for file in sunrise.descripted() {
        println!("{}", Style::new().bold().paint(file.display().to_string()));

        println!("Current Description: {}", sunrise.descriptions[&file]);

        let decision = read_input::InputBuilder::<ReviewDecision>::new()
            .msg("Add a description, press enter to ignore or 'd' to delete: ")
            .get();

        match decision {
            ReviewDecision::Update(string) => sunrise.set_description(&file, string)?,
            ReviewDecision::Delete => sunrise.remove(&file)?,
            _ => {}
        }
    }

    Ok(())
}

#[derive(Debug)]
enum ReviewDecision {
    Update(String),
    Skip,
    Delete
}

impl std::str::FromStr for ReviewDecision {
    type Err = ();

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string.to_lowercase().as_str() {
            "" => Ok(ReviewDecision::Skip),
            "d" => Ok(ReviewDecision::Delete),
            _ => Ok(ReviewDecision::Update(String::from(string)))
        }
    }
}
