fn main() {
    if let Err(e) = bag_address_lookup::create_database() {
        eprintln!("Error creating database: {}", e);
        std::process::exit(1);
    }
}
