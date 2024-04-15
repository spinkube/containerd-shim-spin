const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn print_version() {
    println!("{}", VERSION);
}
