use std::sync::{Arc, Condvar, Mutex};

use bitburner_api::{BitburnerApi, BitburnerError, RemoteClient};

use crate::AppResult;

const NOT_CONNECTED_MESSAGE: &str = "Bitburner is not connected";

#[derive(Clone, Default)]
pub(crate) struct SharedConnection {
    inner: Arc<(Mutex<ConnectionSlot>, Condvar)>,
}

trait SharedClient: BitburnerApi + Send {
    fn close_shared(&mut self) -> bitburner_api::Result<()>;
}

impl SharedClient for RemoteClient {
    fn close_shared(&mut self) -> bitburner_api::Result<()> {
        self.close()
    }
}

#[derive(Default)]
struct ConnectionSlot {
    generation: u64,
    client: Option<Box<dyn SharedClient>>,
    in_use: bool,
}

#[derive(Debug)]
pub(crate) enum SharedConnectionError {
    NotConnected,
    State(String),
    Command(anyhow::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClientDisposition {
    Keep,
    Drop,
}

#[derive(Debug)]
pub(crate) struct ClientCommandResult<T> {
    value: T,
    disposition: ClientDisposition,
}

impl<T> ClientCommandResult<T> {
    pub(crate) fn keep(value: T) -> Self {
        Self {
            value,
            disposition: ClientDisposition::Keep,
        }
    }

    pub(crate) fn drop_client(value: T) -> Self {
        Self {
            value,
            disposition: ClientDisposition::Drop,
        }
    }

    pub(crate) fn keep_client(&self) -> bool {
        self.disposition == ClientDisposition::Keep
    }

    pub(crate) fn into_value(self) -> T {
        self.value
    }
}

impl std::fmt::Display for SharedConnectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConnected => formatter.write_str(NOT_CONNECTED_MESSAGE),
            Self::State(message) => formatter.write_str(message),
            Self::Command(err) => write!(formatter, "{err:#}"),
        }
    }
}

impl std::error::Error for SharedConnectionError {}

impl SharedConnection {
    pub(crate) fn is_connected(&self) -> bool {
        let (lock, _) = &*self.inner;
        match lock.lock() {
            Ok(slot) => slot.client.is_some() || slot.in_use,
            Err(_) => false,
        }
    }

    pub(crate) fn replace(&self, client: RemoteClient) -> bool {
        let previous = self.replace_boxed(Box::new(client));
        if let Some(mut previous) = previous {
            let _ = previous.close_shared();
            true
        } else {
            false
        }
    }

    fn replace_boxed(&self, client: Box<dyn SharedClient>) -> Option<Box<dyn SharedClient>> {
        let (lock, cvar) = &*self.inner;
        let previous = match lock.lock() {
            Ok(mut slot) => {
                slot.generation += 1;
                let previous = slot.client.take();
                slot.client = Some(client);
                slot.in_use = false;
                previous
            }
            Err(_) => {
                eprintln!("error: connection state mutex poisoned");
                return None;
            }
        };
        cvar.notify_all();
        previous
    }

    pub(crate) fn with_client<T>(
        &self,
        command: impl FnOnce(&mut dyn BitburnerApi) -> AppResult<T>,
    ) -> Result<T, SharedConnectionError> {
        self.with_client_control(|api| command(api).map(ClientCommandResult::keep))
    }

    pub(crate) fn with_client_control<T>(
        &self,
        command: impl FnOnce(&mut dyn BitburnerApi) -> AppResult<ClientCommandResult<T>>,
    ) -> Result<T, SharedConnectionError> {
        let (generation, mut remote) = self.take()?;
        let result = command(remote.as_mut());
        let keep = match &result {
            Ok(result) => result.keep_client(),
            Err(err) => !command_error_invalidates_connection(err),
        };
        self.restore_or_close(generation, remote, keep)?;
        result
            .map(ClientCommandResult::into_value)
            .map_err(SharedConnectionError::Command)
    }

    fn take(&self) -> Result<(u64, Box<dyn SharedClient>), SharedConnectionError> {
        let (lock, cvar) = &*self.inner;
        let mut slot = lock
            .lock()
            .map_err(|_| SharedConnectionError::State("connection state mutex poisoned".into()))?;

        while slot.in_use && slot.client.is_none() {
            slot = cvar.wait(slot).map_err(|_| {
                SharedConnectionError::State("connection state mutex poisoned".into())
            })?;
        }

        let Some(remote) = slot.client.take() else {
            return Err(SharedConnectionError::NotConnected);
        };

        slot.in_use = true;
        Ok((slot.generation, remote))
    }

    fn restore_or_close(
        &self,
        generation: u64,
        mut remote: Box<dyn SharedClient>,
        keep: bool,
    ) -> Result<(), SharedConnectionError> {
        if !keep {
            let _ = remote.close_shared();
            self.release_generation(generation)?;
            return Ok(());
        }

        let (lock, cvar) = &*self.inner;
        let mut remote = Some(remote);
        let should_close = {
            let mut slot = lock.lock().map_err(|_| {
                SharedConnectionError::State("connection state mutex poisoned".into())
            })?;
            if slot.generation == generation && slot.client.is_none() {
                slot.client = remote.take();
                slot.in_use = false;
                false
            } else {
                if slot.generation == generation {
                    slot.in_use = false;
                }
                true
            }
        };
        cvar.notify_all();

        if should_close && let Some(mut remote) = remote {
            let _ = remote.close_shared();
        }

        Ok(())
    }

    fn release_generation(&self, generation: u64) -> Result<(), SharedConnectionError> {
        let (lock, cvar) = &*self.inner;
        {
            let mut slot = lock.lock().map_err(|_| {
                SharedConnectionError::State("connection state mutex poisoned".into())
            })?;
            if slot.generation == generation {
                slot.in_use = false;
            }
        }
        cvar.notify_all();
        Ok(())
    }
}

fn command_error_invalidates_connection(err: &anyhow::Error) -> bool {
    err.chain().any(|source| {
        source
            .downcast_ref::<BitburnerError>()
            .is_some_and(bitburner_error_invalidates_connection)
    })
}

pub(crate) fn bitburner_error_invalidates_connection(err: &BitburnerError) -> bool {
    matches!(
        err,
        BitburnerError::InvalidProtocol(_)
            | BitburnerError::Json(_)
            | BitburnerError::Io { .. }
            | BitburnerError::WebSocket { .. }
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::time::Duration;

    use serde_json::{Value, json};

    use super::*;

    #[derive(Clone)]
    struct FakeClient {
        name: &'static str,
        closes: Arc<AtomicUsize>,
    }

    impl FakeClient {
        fn new(name: &'static str) -> (Self, Arc<AtomicUsize>) {
            let closes = Arc::new(AtomicUsize::new(0));
            (
                Self {
                    name,
                    closes: closes.clone(),
                },
                closes,
            )
        }
    }

    impl BitburnerApi for FakeClient {
        fn request_value(
            &mut self,
            _method: &str,
            _params: Option<Value>,
        ) -> bitburner_api::Result<Value> {
            Ok(json!(self.name))
        }
    }

    impl SharedClient for FakeClient {
        fn close_shared(&mut self) -> bitburner_api::Result<()> {
            self.closes.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    fn install_fake(connection: &SharedConnection, client: FakeClient) {
        let previous = connection.replace_boxed(Box::new(client));
        assert!(previous.is_none());
    }

    #[test]
    fn not_connected_returns_not_connected() {
        let connection = SharedConnection::default();

        let err = connection.with_client(|_| Ok(())).expect_err("error");

        assert!(matches!(err, SharedConnectionError::NotConnected));
    }

    #[test]
    fn is_connected_tracks_client_presence() {
        let connection = SharedConnection::default();

        assert!(!connection.is_connected());
        install_fake(&connection, FakeClient::new("one").0);

        assert!(connection.is_connected());
    }

    #[test]
    fn access_is_serialized_while_command_is_in_flight() {
        let connection = SharedConnection::default();
        install_fake(&connection, FakeClient::new("one").0);
        let first_connection = connection.clone();
        let second_connection = connection.clone();
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (second_entered_tx, second_entered_rx) = mpsc::channel();

        let first = std::thread::spawn(move || {
            first_connection.with_client(|_| {
                entered_tx.send(()).expect("send entered");
                release_rx.recv().expect("wait release");
                Ok(())
            })
        });
        entered_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("entered");

        let second = std::thread::spawn(move || {
            second_connection.with_client(|_| {
                second_entered_tx.send(()).expect("send second entered");
                Ok(())
            })
        });

        assert!(
            second_entered_rx
                .recv_timeout(Duration::from_millis(100))
                .is_err()
        );
        release_tx.send(()).expect("release first");
        second_entered_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("second entered after release");

        first.join().expect("first join").expect("first ok");
        second.join().expect("second join").expect("second ok");
    }

    #[test]
    fn replacing_connection_while_command_is_in_flight_keeps_new_client() {
        let connection = SharedConnection::default();
        let (old_client, old_closes) = FakeClient::new("old");
        install_fake(&connection, old_client);
        let in_flight_connection = connection.clone();
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();

        let in_flight = std::thread::spawn(move || {
            in_flight_connection.with_client(|_| {
                entered_tx.send(()).expect("send entered");
                release_rx.recv().expect("wait release");
                Ok(())
            })
        });
        entered_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("entered");

        install_fake(&connection, FakeClient::new("new").0);
        release_tx.send(()).expect("release");
        in_flight
            .join()
            .expect("join")
            .expect("in-flight command ok");

        assert_eq!(old_closes.load(Ordering::SeqCst), 1);
        let value = connection
            .with_client(|api| Ok(api.request_value("who", None)?))
            .expect("new client value");
        assert_eq!(value, json!("new"));
    }

    #[test]
    fn ordinary_command_error_keeps_connection_and_later_calls_do_not_hang() {
        let connection = SharedConnection::default();
        let (client, closes) = FakeClient::new("one");
        install_fake(&connection, client);

        let err = connection
            .with_client::<()>(|_| Err(anyhow::anyhow!("boom")))
            .expect_err("command error");

        assert!(matches!(err, SharedConnectionError::Command(_)));
        assert_eq!(closes.load(Ordering::SeqCst), 0);
        let later = connection
            .with_client(|api| Ok(api.request_value("who", None)?))
            .expect("later call");
        assert_eq!(later, json!("one"));
    }

    #[test]
    fn protocol_command_error_closes_connection_and_later_calls_do_not_hang() {
        let connection = SharedConnection::default();
        let (client, closes) = FakeClient::new("one");
        install_fake(&connection, client);

        let err = connection
            .with_client::<()>(|_| Err(BitburnerError::invalid_protocol("bad response").into()))
            .expect_err("command error");

        assert!(matches!(err, SharedConnectionError::Command(_)));
        assert_eq!(closes.load(Ordering::SeqCst), 1);
        let later = connection
            .with_client(|_| Ok(()))
            .expect_err("not connected");
        assert!(matches!(later, SharedConnectionError::NotConnected));
    }

    #[test]
    fn io_command_error_closes_connection_and_later_calls_do_not_hang() {
        let connection = SharedConnection::default();
        let (client, closes) = FakeClient::new("one");
        install_fake(&connection, client);

        let err = connection
            .with_client::<()>(|_| {
                Err(BitburnerError::io(
                    "read response",
                    std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe broke"),
                )
                .into())
            })
            .expect_err("command error");

        assert!(matches!(err, SharedConnectionError::Command(_)));
        assert_eq!(closes.load(Ordering::SeqCst), 1);
        let later = connection
            .with_client(|_| Ok(()))
            .expect_err("not connected");
        assert!(matches!(later, SharedConnectionError::NotConnected));
    }

    #[test]
    fn control_result_can_drop_connection_without_command_error() {
        let connection = SharedConnection::default();
        let (client, closes) = FakeClient::new("one");
        install_fake(&connection, client);

        let result = connection
            .with_client_control(|_| Ok(ClientCommandResult::drop_client("partial failure")))
            .expect("control result");

        assert_eq!(result, "partial failure");
        assert_eq!(closes.load(Ordering::SeqCst), 1);
        let later = connection
            .with_client(|_| Ok(()))
            .expect_err("not connected");
        assert!(matches!(later, SharedConnectionError::NotConnected));
    }
}
