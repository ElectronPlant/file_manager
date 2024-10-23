//! Testing the file manager menu
use file_manager;



fn main() {
    println!("            --------------------");
    println!("            --- File Manager ---");
    println!("            --------------------\n");
    let dir_vec = Some(Vec::from(["hello/".to_string(), "world/".to_string()]));
    if let Some(s) = file_manager::run_file_naming_menu(true, dir_vec) {
        println!("Selected file name: {}", s);
        file_manager::create_test_file(s);
    }
}
