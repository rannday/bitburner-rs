mod args;
mod cli;
mod connection;
mod fs_sync;
mod http_bridge;
mod ws;

type AppResult<T> = anyhow::Result<T>;

fn main() {
    if let Err(err) = cli::run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}
