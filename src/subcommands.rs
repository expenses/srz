use super::*;

pub fn init(mut path: PathBuf) {
    path.push(".sunrise");

    if path.exists() {
        println!("{} already exists.", path.display());
    } else {
        File::create(&path).unwrap();
        println!("Initialised {}", path.display());
    }
}

pub fn interactive(directory: PathBuf) -> Result<(), Error> {
    let mut sunrise = Sunrise::locate(&directory)?;

    let mut changed = false;

    let paths: Vec<_> = sunrise.files(&directory).collect();

    for path in paths {
        if !sunrise.descriptions.contains_key(&path.to_path_buf()) {
            changed = changed || sunrise.add_description(&path)?;
        }
    }

    changed = changed || sunrise.clean()?;

    if !changed {
        println!("Nothing to do :^)");
    }

    Ok(())
}

pub fn print(directory: PathBuf) -> Result<(), Error> {
    let sunrise = Sunrise::locate(&directory)?;

    for file in sunrise.files(&directory) {
        let indents = file.iter().count() - 1;

        let indent_string: String = std::iter::repeat(" ").take(indents * 2).collect();

        if let Some(desc) = sunrise.descriptions.get(&file) {
            println!("{}// {}", indent_string, Style::new().bold().paint(desc));
        }

        println!("{}{}", indent_string, file.display());
    }

    Ok(())
}

pub fn edit(filename: &Path) -> Result<(), Error> {
    let mut sunrise = Sunrise::locate(filename)?;

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

    sunrise.add_description(&relative);

    Ok(())
}
