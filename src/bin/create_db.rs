fn main() {
    if let Err(e) = bagatel::create_database() {
        eprintln!("Error creating database: {}", e);
        std::process::exit(1);
    }
}
