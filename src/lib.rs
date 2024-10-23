//! File Manager module.
//! The File Manager module is responsible for saving and retrieving the map information so that
//! they can be reused between multiple runs.
//!
//! Version: 0.0 - first version.
//! Version: 1.0 - Adding support for dir changes.

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
                write!(f, "FILE MNG :: Error selected file name is longer than {}.",
                       MAX_FILE_NAME_CHARS),
            Error::InvalidSequentialName =>
                write!(f, "FILE MNG :: Error sequential name count larger than {}.",
                       SEQUENTIAL_FILE_MAX_NUMBER),
            Error::UnknownFileType =>
                write!(f, "FILE MNG :: Error unsupported file type, use {}.",
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

/// Get sub directories in the specified path.
fn get_dir_list(path: &Path) -> Result<Vec<String>> {
    if path.is_dir() {
        Ok(fs::read_dir(path)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_dir())
            .filter_map(|entry| match entry.path().strip_prefix(path) {
                Ok(p) => Some(p.to_path_buf()),
                Err(_) => None
            })
            .map(|entry| entry.to_string_lossy().into_owned())
            .map(|entry|
                if entry.chars().last().unwrap_or_default() == '/' {
                    entry
                } else {
                    format!("{}{}", entry, "/")
                } )
            .collect())
    } else {
        Ok(Vec::new())
    }
}

/// Gets a list of files in the specified path.
fn get_file_list(path: &Path) -> Result<Vec<String>> {
    if path.is_dir() {
        Ok(fs::read_dir(path)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_file())
            .filter(|entry| entry.path().extension().unwrap_or_default().to_str()
                .unwrap_or_default() == DEFAULT_MAP_TYPE)
            .filter_map(|entry| entry.path().file_name()
                .map(|name| name.to_string_lossy().into_owned()))
            .collect())
    } else {
        Ok(Vec::new())
    }
}

/// Prints the list of options in a generic way.
/// The options are numbered and placed in multiple columns so that the user can easily select the
/// desired option.
/// List of notes:
///     1. counter width should match the number of numbers of MAX_SEQUENTIAL_FILE_NUMBER.
///     2. option string width should the maximum allowed size defined by MAX_FILE_NAME_CHARS.
fn print_option_list(opts: &[String], empty_note: &str, start: usize) {
    if opts.is_empty() {
        println!("    {}", empty_note);
    }
    for (cnt, opt) in opts.iter().enumerate() {
        let abs_cnt = cnt + start;
        if cnt % PRINT_COLUMNS == 0 {
            print!("    ");
        }
        print!("{: >3}: {: <30}", abs_cnt, opt); // Notes 1, 2
        if (cnt + 1) % PRINT_COLUMNS == 0 {
            println!();
        }
    }
    if opts.len() % PRINT_COLUMNS != 0 {
        println!();
    }
}

/// List files in the selected directory.
fn print_dir_files(files: &[String], start: usize) {
    print_option_list(files, "(Empty directory)", start);
}

/// Prints the default paths.
/// List of notes:
///     1. Counter width should match the number of numbers of MAX_SEQUENTIAL_FILE_NUMBER.
///     2. File name string width should the maximum allowed size defined by MAX_PATH_NAME_CHARS.
fn print_paths(paths: &[String], start: usize) {
    print_option_list(paths, "(No directories)", start);
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
fn get_sequential_name(current_path:&str, base_name:&str, next:bool) -> Result<String> {
    let mut cnt_max: u16 = 0;
    let mut found: bool = false;
    let path_name = Path::new(&current_path);
    let file_list: Vec<String> = get_file_list(path_name)?;
    for name in file_list
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
// Paths
// ----------------------------------------

/// Initializes the default path list and the current path.
///
/// paths input list of default paths, if None the default path is used.
/// The current path is the first path on the list.
fn init_default_paths(paths: Option<Vec<String>>) -> (String, Vec<String>) {
    let paths: Vec<String> = match paths {
        Some(path_vec) => {
            if path_vec.is_empty() {
                Vec::from([DEFAULT_DIRECTORY.to_string()])
            } else {
                path_vec
            }
        },
        None => Vec::from([DEFAULT_DIRECTORY.to_string()]),
    };
    let default: String = paths[0].clone();
    (default, paths)
}

// ----------------------------------------
// Menus
// ----------------------------------------

fn print_menu_options(current_dir: &str, paths: &[String], sub_paths: &[String], files: &[String]) {
    println!("Input the name of the file to be saved:");
    println!(" - Input a number to preselect a directory or a file.");
    println!(
        " - Input a name ending with / to specify a new absolute or relative (from the execution path) path."
    );
    println!(" - Press CTRL+C to restart the input.");
    println!(" - Press CTRL+D to exit (may need to press CTRL+C first).");
    println!(" - A name ending in _ (e.g. test_), will be transformed into a sequential name.");
    println!("----\nDefault directories (relative):");
    print_paths(paths, 0);
    println!("----\nCurrent dir: {}", current_dir);
    println!("----\nSub dirs:");
    print_paths(sub_paths, paths.len());
    println!("----\nFiles:");
    print_dir_files(files, sub_paths.len() + paths.len());
}

fn check_file_name_len(name: &str) -> Result<()> {
    if name.len() > MAX_FILE_NAME_CHARS {
        Err(Error::InvalidNameTooLong)
    } else {
        Ok(())
    }
}

/// Checks if file exists
fn check_file_exists(path: &str, file_name: String, is_saving:bool) -> Result<String> {
    let full_path: PathBuf = Path::new(path).join(&file_name);
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
                        let new_name: String = get_sequential_name(path, &base_name, true)?;
                        println!("Renaming {} to {}{}",
                                full_path.display(), path.display(), new_name.display());
                        fs::rename(full_path, format!("{}{}", path, new_name))?;
                        return Ok(file_name);
                    },
                    "c" => { // rename new file.
                        let (base_name, _) = file_name.split_once('.')
                            .ok_or(Error::UnknownFileType)?;
                        let base_name = format!("{}{}", base_name, '_');
                        let new_name: String = get_sequential_name(path, &base_name, true)?;
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

/// Checks if the specified directory exists or not.
fn check_dir_exists(path: &str) -> bool {
    Path::new(path).is_dir()
}


/// Launches a menu to ask if yes or no.
///
/// Returns: Ok if the user inputs yes, Error::NeedNewName if the user inputs no, or error code
///          if an error has taken place.
fn ask_yes_no() -> Result<()>{
    let mut rl = rustyline::DefaultEditor::new()?;
    for c in ["y", "yes", "n", "no"] {
        rl.add_history_entry(c)?;
    }
    println!("Input <y>/<yes> or <n>/<no>:");
    loop {
        match rl.readline("> ") {
            Ok(line) => match line.trim() {
                "y" | "yes" => {
                    return Ok(());
                },
               "n" | "no" => {
                    return Err(Error::NeedNewName);
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
                return Err(Error::NeedNewName);
            }
        }
    }
}

/// Checks if an input is a path, a filename or both.
///
/// param line: User input.
///
/// Return tuple:
///     - Path option: if none the dir has not been changed.
///     - Path option: if none there is no valid file name.
fn check_if_path_or_file(line: &str) -> (Option<String>, Option<String>) {
    if line.is_empty() {
        (None, None)
    } else if line.chars().last().unwrap_or_default() == '/' { // Only directory.
        (Some(line.to_string()), None)
    } else {
        match line.rsplit_once('/') {
            Some((d, f)) => (Some(format!("{d}/")), Some(f.to_string())), // Is path with directory.
            None => (None, Some(line.to_string())), // Only file.
        }
    }
}

/// Parses the menu inputs.
/// Outputs:
///     - Path option: if none the dir has not been changed.
///     - Path option: if none there is no valid file name.
fn parse_menu_file(current_path: &str, line: &str,  dirs: &[String], sub_paths: &[String], files: &[String]) -> Result<(Option<String>, Option<String>)> {
    let path: Option<String>;
    let file_name: Option<String>;

    if line.is_empty() {
        // Empty input -> return
        println!("Empty input, try again.");
        file_name = None;
        path = None;
    } else if let Ok(num) = line.parse::<usize>() {
        // Number input --> load existing name.
        if num < dirs.len() {
            if dirs[num].chars().last().unwrap_or_default() == '/' {
                path = Some(dirs[num].to_string());
            } else {
                path = Some(format!("{}{}", dirs[num], "/"));
            }
            file_name = None;
        } else if num - dirs.len() < sub_paths.len() {
            let n = num - dirs.len();
            let p = sub_paths[n].to_string();
            path = Some(format!("{}{}", current_path, p));
            file_name = None;
        } else if num - dirs.len() - sub_paths.len() < files.len() {
            let n = num - dirs.len() - sub_paths.len();
            path = None;
            file_name = Some(is_sequential_name(files[n].to_string()));
        } else {
            path = None;
            file_name = None;
            println!("{} is out of range, try again.", num);
        }
    } else {
        // Path and/or file name.
        (path, file_name) = check_if_path_or_file(line);
    }
    Ok((path, file_name))
}

/// Launches the file name selection menu.
/// If is saving is:
///     True:
///         - Sequential naming will yield the next unused name.
///         - It will fail if the selected name does not exist.
///     False: thus, is loading a file.
///         - Sequential naming will yield the last used name.
///         - If the selected name already exists it will run the rename menu.
fn file_name_menu(current_path: String, paths: &[String], is_saving:bool) -> Result<String> {
    let mut rl = rustyline::DefaultEditor::new()?;
    let mut init_s: String = String::from("");
    //let mut running: bool = true;

    let mut current_path: String = current_path;

    'dir_loop: loop {
        let path_name = Path::new(&current_path);
        let file_list: Vec<String> = get_file_list(path_name)?;
        let sub_paths: Vec<String> = get_dir_list(path_name)?;
        print_menu_options(&current_path, paths, &sub_paths, &file_list);

        rl.clear_history()?;
        for f in file_list.iter().rev() {
            rl.add_history_entry(f)?;
        }
        for p in paths.iter().rev() {
            rl.add_history_entry(p)?;
        }

        'file_loop: loop {
            let readline =
                rl.readline_with_initial("> ", (&init_s, ""));

            match readline {
                Ok(line) => {
                    let line: String = line.split(' ')
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                        .join("_");
                    let l: &str = &line;
                    rl.add_history_entry(&line)?;

                    let (path, file): (Option<String>, Option<String>) =
                        parse_menu_file( &current_path, l, paths, &sub_paths, &file_list)?;

                    let path_updated: bool;
                    let path = match path {
                        Some(p) => p,
                        None => current_path.clone(),
                    };
                    if !check_dir_exists(&path) {
                        // Selected path does not exist.
                        println!("Selected path does not exists: {}", &path);
                        if is_saving {
                            // ask if the new dir needs to be created or not.
                            println!("Create new dir?");
                            match ask_yes_no() {
                                Ok(()) => {
                                    current_path = path;
                                    let new_dir_path = Path::new(&current_path);
                                    fs::create_dir_all(new_dir_path)?;
                                    path_updated = true;
                                },
                                Err(Error::NeedNewName) => {
                                    println!("New directory not created, input a new one.");
                                    continue 'file_loop;
                                }
                                Err(e) => return Err(e),
                            };
                        } else {
                            continue 'file_loop;
                        }
                    } else {
                        current_path = path;
                        path_updated = true;
                    }

                    if let Some(mut file) = file {
                        // check sequential naming
                        if file.chars().last().unwrap_or_default() == SEQUENTIAL_NAMING_CHAR {
                            println!("Getting sequential name...");
                            file = get_sequential_name(&current_path, &file, is_saving)?;
                        }
                        // Check extension
                        let file: String = match file.split_once('.') {
                            Some((s, ext)) => {
                                if ext == DEFAULT_MAP_TYPE {
                                    file
                                } else {
                                    println!("{}", Error::UnknownFileType);
                                    init_s = format!("{s}.{DEFAULT_MAP_TYPE}");
                                    if path_updated {
                                        continue 'dir_loop;
                                    } else {
                                        continue 'file_loop;
                                    }
                                }
                            },
                            None => {
                                format!("{}.{}", file, DEFAULT_MAP_TYPE)
                            }
                        };
                        // Name length
                        check_file_name_len(&file)?;
                        // Check if file exists
                        let file = match check_file_exists(&current_path, file, is_saving) {
                            Ok(s) => s,
                            Err(Error::NeedNewName) => {
                                init_s.clear();
                                println!("Please input a new name.");
                                if path_updated {
                                    continue 'dir_loop;
                                } else {
                                    continue 'file_loop;
                                }
                            },
                            Err(e) => return Err(e),
                        };
                        return Ok(format!("{}{}", current_path, file));
                    }
                    init_s.clear();
                    if path_updated {
                        continue 'dir_loop;
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
    }
}

/// Intermediate function to handle errors in the save file menu.
/// If is_saving is true, it will run the file saving option; otherwise it will run the load
/// file option.
///
fn run_save_file_menu_with_errors(is_saving: bool, default_dirs: Option<Vec<String>>) -> Result<String> {
    let (default_path, paths) = init_default_paths(default_dirs);
    let full_path: String = file_name_menu(default_path, &paths, is_saving)?;
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
pub fn run_file_naming_menu(is_saving: bool, default_dirs: Option<Vec<String>>) -> Option<String> {
    match run_save_file_menu_with_errors(is_saving, default_dirs) {
        Err(e) => {
            println!("{e}");
            None
        }
        Ok(s) => Some(s)
    }
}

/// Creates a test file to test the crate.
pub fn create_test_file(file_path: String) {
    match fs::write(file_path, "This is just a test file, please delete.") {
        Ok(_) => println!("File created!"),
        Err(e) => println!("Failed to crate file {e}"),
    }
}
