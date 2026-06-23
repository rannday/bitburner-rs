use std::net::TcpListener;

use tungstenite::accept;
use tungstenite::protocol::Message;

use crate::error::{AppError, AppResult};

pub fn serve(address: &str) -> AppResult<()> {
    let listener = TcpListener::bind(address)?;
    println!("listening on {address}");
    println!("waiting for Bitburner Remote API client");
    let (stream, peer) = listener.accept()?;
    println!("client connected from {peer}");
    let mut socket = accept(stream)
        .map_err(|err| AppError::Remote(format!("websocket handshake failed: {err}")))?;
    println!("websocket connected");
    println!("serve mode is connection-only for now; IPC for separate bbrs commands is TODO");

    loop {
        match socket.read()? {
            Message::Text(text) => println!("{text}"),
            Message::Binary(bytes) => println!("binary message: {} bytes", bytes.len()),
            Message::Ping(bytes) => socket.send(Message::Pong(bytes))?,
            Message::Pong(_) => {}
            Message::Close(_) => {
                println!("client disconnected");
                return Ok(());
            }
            Message::Frame(_) => {}
        }
    }
}
