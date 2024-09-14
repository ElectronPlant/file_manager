//! File Manager module.
//! The File Manager module is responsible for saving and retrieving the map information so that
//! they can be reused between multiple runs.
//!
//! Version: 0.0 - first version
//!

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::{result, fmt};

use rustyline::completion::Candidate;
use rustyline::error::ReadlineError;

use thiserror::Error;

// --------------------------------------------------------------------------------
// Definitions
// --------------------------------------------------------------------------------

/// The file is executed from where the cargo run is called.
/// This will assume that the cargo run is called from the main project dir.
const DEFAULT_DIRECTORY: &str = "./test_dir/";

const DEFAULT_MAP_TYPE: &str = "map"; // Do not add the period for the extension.
const DIR_CHANGE_STR: &str = "dir";

const SEQUENTIAL_FILE_PADDING_LEN: usize = 3;
const SEQUENTIAL_NAMING_CHAR: char = '_';
const SEQUENTIAL_FILE_MAX_NUMBER: u16 = 999; // Note that the number of digits should match the
                                             // digits defined in print_dir_files and path_from_cnt
const PRINT_COLUMNS: usize = 4;
const MAX_FILE_NAME_CHARS: usize = 30; // Note that this should match the print in print_dir_files.

#[derive(Error, Debug)]
enum Error {
    /// External errors
    Io(#[from] io::Error),
    Cmd(#[from] rustyline::error::ReadlineError),

    /// Manually terminated, not really an error but useful to state no file has been selected.
    ManuallyTerminated,

    /// Need new name, not really an error but useful to state that the name menu needs to re-run.
    NeedNewName,

    /// Delete file, not really an error bur useful when the intention is just to delete a file.
    FileDeletion,

    /// Custom errors.
    InvalidNameTooLong,
    InvalidSequentialName,
    UnknownFileType,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref err) => err.fmt(f),
            Error::Cmd(ref err) => err.fmt(f),
            Error::ManuallyTerminated =>
                write!(f, "FILE MNG :: File selection has been manually terminated with CTRL+D"),
            Error::NeedNewName =>
                write!(f, "FILE MNG :: File selection needs to be re-run."),
            Error::FileDeletion =>
                write!(f, "FILE MNG :: Specified file has been deleted."),
            Error::InvalidNameTooLong =>
                write!(f, "FILE MNG :: Error selected file name is longer than {}",
                       MAX_FILE_NAME_CHARS),
            Error::InvalidSequentialName =>
                write!(f, "FILE MNG :: Error sequential name count too larger than {}",
                       SEQUENTIAL_FILE_MAX_NUMBER),
            Error::UnknownFileType =>
                write!(f, "FILE MNG :: Error unsupported file type, use {}",
                       DEFAULT_MAP_TYPE),
        }
    }
}

type Result<T> = result::Result<T, Error>;

// --------------------------------------------------------------------------------
// Implementations
// --------------------------------------------------------------------------------

// ----------------------------------------
// Path handling
// ----------------------------------------
/// Checks if the path exists and creates it if it doesn't.
fn handle_path(path: &str) -> Result<PathBuf> {
    let path_name = Path::new(path);
    if !path_name.exists() {
        fs::create_dir_all(path_name)?;
        if let Some(abs_path) = fs::canonicalize(path_name)?.to_str() {
            println!("FILE MNG :: Map dir not found, creating <{}>.", abs_path);
        } else {
            println!("FILE MNG :: Unable to show created file.");
        }
    }
    Ok(path_name.to_path_buf())
}

fn get_file_list(path: &Path) -> Result<Vec<String>> {
    Ok(fs::read_dir(path)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .filter(|entry| entry.path().extension().unwrap_or_default().to_str()
            .unwrap_or_default() == DEFAULT_MAP_TYPE)
        .filter_map(|entry| entry.path().file_name()
            .map(|name| name.to_string_lossy().into_owned()))
        .collect())
}

/// List files in the selected directory.
/// Note that this function assumes that the directory has been selected and already created.
/// List of notes:
///     1. counter width should match the number of numbers of MAX_SEQUENTIAL_FILE_NUMBER.
///     2. file_name width should match the maximum allowed size defined by MAX_FILE_NAME_CHARS.
///
fn print_dir_files(files: &[String]) {
    if files.is_empty() {
        println!("    (empty directory)");
    }
    for (cnt, file) in files.iter().enumerate() {
        if cnt % PRINT_COLUMNS == 0 {
            print!("    ");
        }
        print!("{: >3}: {: <30}", cnt, file); // Notes 1, 2
        if (cnt + 1) % PRINT_COLUMNS == 0 {
            println!();
        }
    }
    if files.len() % PRINT_COLUMNS != 0 {
        println!();
    }
}

// ----------------------------------------
// Sequential naming
// ----------------------------------------

/// Gets the sequential name of the file from its base name and the current count.
/// Note that the base_name will already have the trailing "_", so there is no need to add it.
fn get_sequential_name_from_count(base_name: &str, cnt: u16) -> String {
    format!("{}{:0>3}.{}", base_name, cnt, DEFAULT_MAP_TYPE)
}

/// Searches the files to get the next sequential name.
/// if next is true the next unused name is returned; otherwise the last used name.
fn get_sequential_name(files:&[String], base_name:&str, next:bool) -> Result<String> {
    let mut cnt_max: u16 = 0;
    let mut found: bool = false;
    for name in files
        .iter()
        .filter(|entry|
            &entry[0..entry.len()-(DEFAULT_MAP_TYPE.len() + 1 + SEQUENTIAL_FILE_PADDING_LEN)] ==
            base_name)
    {
        let cnt = name
            .split(SEQUENTIAL_NAMING_CHAR).last().unwrap_or_default()
            .split('.').next().unwrap_or_default();
        if let Ok(cnt) = cnt.parse::<u16>() {
            found = true;
            if cnt > cnt_max {
                cnt_max = cnt;
            }
        }
    }
    if next && found {
        cnt_max += 1;
    }

    if cnt_max <= SEQUENTIAL_FILE_MAX_NUMBER {
        Ok(get_sequential_name_from_count(base_name, cnt_max))
    } else {
        Err(Error::InvalidSequentialName)
    }
}

/// If the name is sequential, return basename only.
/// Sequential names end in <base_name>_XXX.<extension>.
///
fn is_sequential_name(file_name: String) -> String {
    let (base_name, _) = file_name.split_once('.').unwrap_or_default();
    if let Some(cnt) = base_name.split('_').last() {
        if cnt.len() == SEQUENTIAL_FILE_PADDING_LEN && cnt.parse::<u16>().is_ok() {
            let last_index: usize = file_name.len() - (DEFAULT_MAP_TYPE.len() + 1 + SEQUENTIAL_FILE_PADDING_LEN);
            return file_name[0..(last_index)].to_string();
        }
    }
    file_name
}

// ----------------------------------------
// Menus
// ----------------------------------------

fn print_menu_options(path_name: &str, files: &[String]) {
    println!("Input the name of the file to be saved:");
    println!(" - Input a number to preselect the file.");
    println!(" - Input 'dir' to change directory (TODO).");
    println!(" - Press CTRL+C to restart the input.");
    println!(" - Press CTRL+D to exit (may need to press CTRL+C first).");
    println!(" - A name ending in _ (e.g. test_), will be transformed into a sequential name.");
    println!("Current dir: {}", path_name.to_owned());
    print_dir_files(files);
}

fn check_file_name_len(name: &str) -> Result<()> {
    if name.len() > MAX_FILE_NAME_CHARS {
        Err(Error::InvalidNameTooLong)
    } else {
        Ok(())
    }
}

/// Checks if file exists
fn check_file_exists(path: &Path, file_name: String, files: &[String], is_saving:bool) -> Result<String> {
    let full_path: PathBuf = path.join(&file_name);
    if full_path.is_file() && is_saving {
        println!("FILE MNG :: file {} already exits while saving.", full_path.to_string_lossy());
        println!("Input:");
        println!("  'r' to replace existing file.");
        println!("  'm' to turn existing file into sequential naming.");
        println!("  'c' to turn new file into sequential naming.");
        println!("  'n' to select a new name.");
        println!("  'd' to delete the specified file.");

        // run editor:
        let mut rl = rustyline::DefaultEditor::new()?;
        for c in ["r", "m", "c", "n", "d"] {
            rl.add_history_entry(c)?;
        }
        loop {
            match rl.readline("> ") {
                Ok(line) => match line.trim() {
                    "r" => { // Replace
                        println!("Replacing {}...", path.display());
                        fs::remove_file(full_path)?;
                        return Ok(file_name);
                    },
                    "m" => { // Move old file.
                        let (base_name, _) = file_name.split_once('.')
                            .ok_or(Error::UnknownFileType)?;
                        let base_name = format!("{}{}", base_name, '_');
                        let new_name: String = get_sequential_name(files, &base_name, true)?;
                        println!("Renaming {} to {}{}",
                                full_path.display(), path.display(), new_name.display());
                        fs::rename(full_path, path.join(new_name))?;
                        return Ok(file_name);
                    },
                    "c" => { // rename new file.
                        let (base_name, _) = file_name.split_once('.')
                            .ok_or(Error::UnknownFileType)?;
                        let base_name = format!("{}{}", base_name, '_');
                        let new_name: String = get_sequential_name(files, &base_name, true)?;
                        return Ok(new_name);
                    },
                    "n" => {
                        return Err(Error::NeedNewName);
                    },
                    "d" => {
                        fs::remove_file(&full_path)?;
                        println!("File {} has been deleted.", full_path.display());
                        return Err(Error::FileDeletion);
                    },
                    _ => println!("Invalid input, try again."),
                },
                Err(ReadlineError::Interrupted) => { // CTRL+C
                    return Err(Error::NeedNewName);
                },
                Err(ReadlineError::Eof) => { // CTRL+D
                    return Err(Error::ManuallyTerminated);
                },
                Err(err) => {
                    println!("FILE MNG :: ERROR :: failed due to {err}");
                }
            }
        }
    } else if !full_path.is_file() && !is_saving {
        println!("FILE MNG :: file {} does not exists while loading.", full_path.to_string_lossy());
        return Err(Error::NeedNewName);
    }
    Ok(file_name)
}

fn parse_menu_file(line: &str, files: &[String], is_saving:bool) -> Result<(String, bool)> {
    let mut ret: String = String::from("");
    let mut running: bool = true;

    if let Ok(num) = line.parse::<usize>() {
        // Number input --> load existing name.
        if num > files.len() {
            println!("{} is not available.", num);
        } else {
            ret = files[num].to_owned();
            ret = is_sequential_name(ret);
        }
    } else if line == DIR_CHANGE_STR {
        // Dir change input.
        println!("Change directory: TODO, function not supported.");
        ret.clear();
    } else if line.chars().last().unwrap_or_default() == SEQUENTIAL_NAMING_CHAR {
        // Sequential naming input --> Get sequential name.
        println!("Getting sequential name...");
        ret = get_sequential_name(files, line, is_saving)?;
    } else {
        // Actual name input --> return name.
        running = false;
        ret = match line.split_once('.') {
            Some((s, ext)) => {
                if ext == DEFAULT_MAP_TYPE {
                    line.to_string()
                } else {
                    println!("{}", Error::UnknownFileType);
                    running = true;
                    format!("{s}.{DEFAULT_MAP_TYPE}")
                }
            },
            None => {
                format!("{}.{}", line, DEFAULT_MAP_TYPE)
            }
        };
        check_file_name_len(&ret)?;
    }
    Ok((ret, running))
}

/// Launches the file name selection menu.
/// If is saving is:
///     True:
///         - Sequential naming will yield the next unused name.
///         - It will fail if the selected name does not exist.
///     False: thus, is loading a file.
///         - Sequential naming will yield the last used name.
///         - If the selected name already exists it will run an overwrite/fs::rename menu.
///
fn file_name_menu(path: PathBuf, files: &Vec<String>, is_saving:bool) -> Result<String> {
    let mut rl = rustyline::DefaultEditor::new()?;
    let mut init_s: String = String::from("");
    let mut running: bool = true;

    print_menu_options(&path.to_string_lossy(), files);

    for file in files {
        rl.add_history_entry(file)?;
    }

    while running {
        let readline =
            rl.readline_with_initial("> ", (&init_s, ""));

        match readline {
            Ok(line) => {
                let line: String = line.split(' ')
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join("_");
                let l: &str = &line;
                rl.add_history_entry(line.as_str())?;
                (init_s, running) = match parse_menu_file(l, files, is_saving) {
                    Ok((s, r)) => (s, r),
                    Err(e) => {
                        println!("{e}");
                        (l.to_string(), true)
                    }
                };

                if !running {
                    init_s = match check_file_exists(&path, init_s, files, is_saving) {
                        Ok(s) => s,
                        Err(Error::NeedNewName) => {
                            running = true;
                            println!("Please select a new file name.");
                            print_menu_options(&path.to_string_lossy(), files);
                            "".to_string()
                        },
                        Err(e) => return Err(e),
                    }
                }
            },
            Err(ReadlineError::Interrupted) => { // CTRL+C
                init_s.clear();
            },
            Err(ReadlineError::Eof) => { // CTRL+D
                return Err(Error::ManuallyTerminated);
            },
            Err(err) => {
                println!("FILE MNG :: ERROR :: failed due to {err}");
            }
        }
    }
    Ok(init_s)
}

/// Intermediate function to handle errors in the save file menu.
/// If is_saving is true, it will run the file saving option; otherwise it will run the load
/// file option.
///
fn run_save_file_menu_with_errors(is_saving: bool) -> Result<String> {
    let path_name = DEFAULT_DIRECTORY;
    let path: PathBuf = handle_path(path_name)?;
    let file_list: Vec<String> = get_file_list(&path)?;

    let file_name: String = file_name_menu(path, &file_list, is_saving)?;
    let full_path: String = format!("{}{}", path_name, file_name);

    Ok(full_path)
}

// ----------------------------------------
// Mains
// ----------------------------------------

/// Runs the file naming menu.
///
/// All errors are handled internally for simplicity.
///
/// \param is_saving: if true serves the file save menu; otherwise it serves the load file menu.
/// \return: option with the selected file name, None if error took place or it was canceled.
///
pub fn run_file_naming_menu(is_saving: bool) -> Option<String> {
    match run_save_file_menu_with_errors(is_saving) {
        Err(e) => {
            println!("{e}");
            None
        }
        Ok(s) => Some(s)
    }
}

/// Creates a test file to test the crate.
pub fn create_file(file_path: String) {
    match fs::write(file_path, "This is just a test file, please delete.") {
        Ok(_) => println!("File created!"),
        Err(e) => println!("Failed to crate file {e}"),
    }
}