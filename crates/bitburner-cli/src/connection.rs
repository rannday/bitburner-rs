use std::sync::{Arc, Condvar, Mutex};

use bitburner_api::{BitburnerApi, RemoteClient};

use crate::AppResult;

const NOT_CONNECTED_MESSAGE: &str = "Bitburner is not connected";

#[derive(Clone, Default)]
pub(crate) struct SharedConnection {
    inner: Arc<(Mutex<ConnectionSlot>, Condvar)>,
}

#[derive(Default)]
struct ConnectionSlot {
    generation: u64,
    client: Option<RemoteClient>,
    in_use: bool,
}

#[derive(Debug)]
pub(crate) enum SharedConnectionError {
    NotConnected,
    State(String),
    Command(anyhow::Error),
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

    pub(crate) fn replace(&self, client: RemoteClient) -> Option<RemoteClient> {
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
        let (generation, mut remote) = self.take()?;
        let result = command(&mut remote);
        let keep = result.is_ok();
        self.restore_or_close(generation, remote, keep)?;
        result.map_err(SharedConnectionError::Command)
    }

    fn take(&self) -> Result<(u64, RemoteClient), SharedConnectionError> {
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
        mut remote: RemoteClient,
        keep: bool,
    ) -> Result<(), SharedConnectionError> {
        if !keep {
            let _ = remote.close();
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
            let _ = remote.close();
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
