mod args;
mod cli;
mod fs_sync;
mod ws;

type AppResult<T> = anyhow::Result<T>;

fn main() {
    if let Err(err) = cli::run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}
