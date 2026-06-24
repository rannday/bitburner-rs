fn main() {
    if let Err(err) = bitburner_rs::run_cli() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}
