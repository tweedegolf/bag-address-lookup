#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.len() == 2 {
        let postal_code = &args[0];
        let house_number: u32 = match args[1].parse() {
            Ok(value) => value,
            Err(_) => {
                eprintln!("Invalid house number: {}", args[1]);
                std::process::exit(1);
            }
        };

        let database = match bag_address_lookup::DatabaseHandle::load() {
            Ok(database) => database,
            Err(err) => {
                eprintln!("Error loading database: {}", err);
                std::process::exit(1);
            }
        };

        if let Some((public_space, locality)) = database.lookup(postal_code, house_number) {
            print!("{public_space}\n{locality}\n");
        } else {
            eprintln!("No address found for {postal_code} {house_number}");
            std::process::exit(1);
        }

        return;
    }

    let addr = args
        .into_iter()
        .next()
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    println!("Starting BAG webservice on {}", addr);

    if let Err(e) = bag_address_lookup::serve(&addr).await {
        eprintln!("Error running service: {}", e);
        std::process::exit(1);
    }
}
