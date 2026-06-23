mod args;
mod error;
mod fs_sync;
mod path;
mod remote;
mod ws;

use args::Command;
use error::{AppError, AppResult};
use remote::{DEFAULT_ADDRESS, RemoteClient};

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> AppResult<()> {
    let command = args::parse_env()?;

    match command {
    Command::Help => {
      print_help();
      Ok(())
    }
    Command::Version => {
      println!("bbrs {}", env!("CARGO_PKG_VERSION"));
      Ok(())
    }
    Command::Serve => ws::serve("127.0.0.1:12525"),
    Command::Mcp => Err(AppError::NotImplemented(
      "mcp command not implemented yet; future Zed integration should call bbrs sync or bbrs mcp"
        .to_string(),
    )),
    Command::Files { server } => {
      let mut remote = RemoteClient::listen(DEFAULT_ADDRESS)?;
      for file in remote.get_file_names(&server)? {
        println!("{file}");
      }
      Ok(())
    }
    Command::Get {
      server,
      filename,
      local_path,
    } => {
      let mut remote = RemoteClient::listen(DEFAULT_ADDRESS)?;
      let content = remote.get_file(&server, &filename)?;
      if let Some(path) = local_path {
        std::fs::write(path, content)?;
      } else {
        print!("{content}");
      }
      Ok(())
    }
    Command::Push {
      server,
      remote_filename,
      local_path,
    } => {
      let content = std::fs::read_to_string(local_path)?;
      let mut remote = RemoteClient::listen(DEFAULT_ADDRESS)?;
      remote.push_file(&server, &remote_filename, &content)?;
      println!("{remote_filename}");
      Ok(())
    }
    Command::Delete { server, filename } => {
      let mut remote = RemoteClient::listen(DEFAULT_ADDRESS)?;
      remote.delete_file(&server, &filename)?;
      println!("{filename}");
      Ok(())
    }
    Command::Metadata { server, filename } => {
      let mut remote = RemoteClient::listen(DEFAULT_ADDRESS)?;
      let metadata = remote.get_file_metadata(&server, &filename)?;
      println!("{}", serde_json::to_string_pretty(&metadata)?);
      Ok(())
    }
    Command::AllFiles { server, local_path } => {
      let mut remote = RemoteClient::listen(DEFAULT_ADDRESS)?;
      let files = remote.get_all_files(&server)?;
      std::fs::write(local_path, serde_json::to_string_pretty(&files)?)?;
      Ok(())
    }
    Command::AllMetadata { server } => {
      let mut remote = RemoteClient::listen(DEFAULT_ADDRESS)?;
      let metadata = remote.get_all_file_metadata(&server)?;
      println!("{}", serde_json::to_string_pretty(&metadata)?);
      Ok(())
    }
    Command::Ram { server, filename } => {
      let mut remote = RemoteClient::listen(DEFAULT_ADDRESS)?;
      println!("{}", remote.calculate_ram(&server, &filename)?);
      Ok(())
    }
    Command::Defs { local_path } => {
      let mut remote = RemoteClient::listen(DEFAULT_ADDRESS)?;
      let content = remote.get_definition_file()?;
      if let Some(path) = local_path {
        std::fs::write(path, content)?;
      } else {
        print!("{content}");
      }
      Ok(())
    }
    Command::Save { local_path } => {
      let mut remote = RemoteClient::listen(DEFAULT_ADDRESS)?;
      let save = remote.get_save_file()?;
      std::fs::write(local_path, serde_json::to_string_pretty(&save)?)?;
      Ok(())
    }
    Command::Sync(options) => {
      let plan = fs_sync::build_sync_plan(&options.local_dir, options.remote_dir.as_deref())?;

      if options.dry_run {
        println!(
          "sync dry-run server={} local={} remote-dir={} clean={}",
          options.server,
          options.local_dir.display(),
          options.remote_dir.as_deref().unwrap_or(""),
          options.clean
        );
        for item in plan {
          println!("{} -> {}", item.local_path.display(), item.remote_path);
        }
        return Ok(());
      }

      if options.clean {
        return Err(AppError::NotImplemented(
          "sync --clean is TODO: dry-run works, upload works without clean".to_string(),
        ));
      }
      let mut remote = RemoteClient::listen(DEFAULT_ADDRESS)?;
      for item in plan {
        let content = std::fs::read_to_string(&item.local_path)?;
        remote.push_file(&options.server, &item.remote_path, &content)?;
        println!("uploaded {}", item.remote_path);
      }
      Ok(())
    }
    Command::Clean { server } => {
      let mut remote = RemoteClient::listen(DEFAULT_ADDRESS)?;
      remote.clean_server(&server)
    }
  }
}

fn print_help() {
    println!("bbrs - Bitburner Remote API sync tool");
    println!();
    println!("Commands:");
    println!("  help");
    println!("  version");
    println!("  serve");
    println!("  files [server]");
    println!("  get <server> <filename> [local-path]");
    println!("  push <server> <remote-filename> <local-path>");
    println!("  delete <server> <filename>");
    println!("  metadata <server> <filename>");
    println!("  all-files [server] <local-path>");
    println!("  all-metadata [server]");
    println!("  ram <server> <filename>");
    println!("  defs [local-path]");
    println!("  save <local-path>");
    println!("  sync [local-dir] [remote-dir] [--server <server>] [--clean] [--dry-run]");
    println!("  clean [server]");
    println!("  mcp");
    println!();
    println!("Sync uploads .js, .ts, .txt, .script, and .json files only.");
    println!(
        "Sync skips default generated/VCS/editor dirs: .git, target, node_modules, dist, build, .zed, .vscode, .idea, coverage, tmp, temp."
    );
    println!("Remote paths preserve paths relative to <local-dir>, then prefix [remote-dir].");
    println!("Windows backslashes become Bitburner forward slashes.");
    println!(
        "Non-dry-run sync listens on 127.0.0.1:12525, waits for Bitburner, uploads, then exits."
    );
}
