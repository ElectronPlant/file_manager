//! Testing the file manager menu
use file_manager;

fn main() {
    println!("            --------------------");
    println!("            --- File Manager ---");
    println!("            --------------------\n");
    if let Some(s) = file_manager::run_file_naming_menu(true) {
        println!("Selected file name: {}", s);
        file_manager::create_file(s);
    }
}
