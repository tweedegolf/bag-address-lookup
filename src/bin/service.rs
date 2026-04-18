#[cfg(feature = "cli")]
use bag_address_lookup::DatabaseHandle;

const VERSION_TEXT: &str = concat!(
    "BAG Address Lookup Service version ",
    env!("CARGO_PKG_VERSION")
);

fn is_version_flag(arg: &str) -> bool {
    arg == "--version" || arg == "-v"
}

#[cfg(feature = "cli")]
fn load_database() -> DatabaseHandle {
    match DatabaseHandle::load() {
        Ok(database) => database,
        Err(err) => {
            eprintln!("Error loading database: {}", err);
            std::process::exit(1);
        }
    }
}

#[cfg(feature = "cli")]
fn cmd_lookup(postal_code: &str, house_number_arg: &str) -> i32 {
    let house_number: u32 = match house_number_arg.parse() {
        Ok(value) => value,
        Err(_) => {
            eprintln!("Invalid house number: {}", house_number_arg);
            return 1;
        }
    };

    let database = load_database();

    if let Some((public_space, locality)) = database.lookup(postal_code, house_number) {
        println!("{public_space}\n{locality}");
        0
    } else {
        eprintln!("No address found for {postal_code} {house_number}");
        1
    }
}

#[cfg(feature = "cli")]
fn cmd_list_localities() -> i32 {
    let database = load_database();
    for (wp, wp_code, gm, gm_code, pv, _unique, _had_suffix) in database.locality_details() {
        println!("{wp}\t{wp_code}\t{gm}\t{gm_code}\t{pv}");
    }
    0
}

#[cfg(feature = "cli")]
fn cmd_list_municipalities() -> i32 {
    let database = load_database();
    for (gm, gm_code, pv, _unique, _had_suffix) in database.municipality_details() {
        println!("{gm}\t{gm_code}\t{pv}");
    }
    0
}

/// Try to run a CLI command. Returns `Some(exit_code)` if the args matched a
/// CLI command, `None` otherwise.
#[cfg(feature = "cli")]
fn try_run_cli(args: &[String]) -> Option<i32> {
    match args.first().map(String::as_str) {
        Some("list-localities") if args.len() == 1 => Some(cmd_list_localities()),
        Some("list-municipalities") if args.len() == 1 => Some(cmd_list_municipalities()),
        _ if args.len() == 2 => Some(cmd_lookup(&args[0], &args[1])),
        _ => None,
    }
}

#[cfg(feature = "webservice")]
async fn run_server(args: &[String]) -> i32 {
    let addr = args
        .first()
        .cloned()
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    println!("Starting BAG webservice on {}", addr);

    if let Err(e) = bag_address_lookup::serve(&addr).await {
        eprintln!("Error running service: {}", e);
        return 1;
    }
    0
}

#[cfg(not(feature = "webservice"))]
fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  bag-service --version");
    #[cfg(feature = "cli")]
    {
        eprintln!("  bag-service <postal_code> <house_number>");
        eprintln!("  bag-service list-localities");
        eprintln!("  bag-service list-municipalities");
    }
}

#[cfg(feature = "webservice")]
#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.len() == 1 && is_version_flag(&args[0]) {
        println!("{VERSION_TEXT}");
        return;
    }

    #[cfg(feature = "cli")]
    if let Some(code) = try_run_cli(&args) {
        std::process::exit(code);
    }

    std::process::exit(run_server(&args).await);
}

#[cfg(all(not(feature = "webservice"), feature = "cli"))]
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.len() == 1 && is_version_flag(&args[0]) {
        println!("{VERSION_TEXT}");
        return;
    }

    if let Some(code) = try_run_cli(&args) {
        std::process::exit(code);
    }

    print_usage();
    std::process::exit(1);
}

#[cfg(not(any(feature = "webservice", feature = "cli")))]
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.len() == 1 && is_version_flag(&args[0]) {
        println!("{VERSION_TEXT}");
        return;
    }

    eprintln!(
        "bag-service was built without the 'cli' or 'webservice' features enabled; nothing to do."
    );
    print_usage();
    std::process::exit(1);
}
