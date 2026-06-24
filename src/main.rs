fn main() {
    if let Err(err) = bitburner_rs::cli::run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}
