# File Manager

Simple file manager module for CLI applications in rust.

## Brief
This module handles the naming selection for saving and loading files using a command line
interface. Taking care off the error handling, directory creation, file checking etc.
This is an ongoing work, so not all features are not yet implemented.

## API
### Launch file name selection menu
There is only one function call required to launch the file name selection menu:

```
pub fn run_file_naming_menu(is_saving: bool) -> Option<String>
```

The argument of the function is used to specify if we are creating (if set to true), or loading
(if set to false) a file.

This function returns an Option.
 * None represent that an error has taken place, so the file name selection could not be completed.
 * Otherwise the file, along with its path, is returned.

### Testing functions
```
pub fn create_file(file_path: String)
```
The create file function is a placeholder to easily test the correct functionality of this module.

## Sequential naming
this feature is used to simplify version control in the generated files. sequential names are
generated with an incremental postfix at the end. This postfix is incremented as new files are
being created. This is done by adding a underscore ('_') followed by a three digit, zero padded
number to the name. E.g. "test.map" would be converted into "test_000.map" in sequential naming. If
a new file is created the name of the new file would be "test_001.map".

There are two way to create sequential names:
 1. In the file selection menu inputting a name ending in "_" (e.g. "test_").
 2. Using the number-based loader to select a sequence file, this will automatically load the base
 name with the '_' ending.
 3. Selecting a name that already exists, while saving a file. There is an option to keep the base
 name for the current file, and turn the existing name into sequential numbering, or to save
 the new file with this numbering.


# Acknowledgements

Command lines input reading is implemented using the [RustyLine](https://github.com/kkawakam/rustyline) crate.

Internal errors use the [ThisError](https://github.com/dtolnay/thiserror) crate.
