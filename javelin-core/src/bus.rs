// TODO: allowed to reduce noise, remove later
// #![allow(dead_code)]
// #![allow(unused_imports)]


mod error;
mod common;
mod message;
mod registry;
mod connection;


use {
    std::convert::TryInto,
    tokio::{
        sync::{mpsc, RwLock},
        stream::StreamExt,
    },
    self::{
        common::{BusName, Request, response_channel},
        registry::Registry,
    },
};

pub use self::{
    error::Error,
    message::Message,
    connection::{Connection, Addr},
};


type Incoming = mpsc::UnboundedReceiver<Request>;
type RawHandle = mpsc::UnboundedSender<Request>;


pub struct Bus {
    registry: RwLock<Registry>,
    incoming: Incoming,
    handle: RawHandle,
}

impl Bus {
    pub fn new() -> Self {
        let registry = RwLock::new(Registry::new());
        let (handle, incoming) = mpsc::unbounded_channel();
        Self { registry, incoming, handle }
     }

    /// Get a new bus handle to register an address.
    pub fn get_handle(&self) -> Handle {
        Handle { bus: self.handle.clone() }
    }

    pub fn spawn() -> Handle {
        let bus = Self::new();
        let handle = bus.get_handle();
        tokio::spawn(bus.listen());
        handle
    }

    /// Start listening to bus messages.
    async fn listen(mut self) {
        while let Some(request) = self.next_message().await {
            match request {
                Request::Message(message, responder) => {
                    let registry = self.registry.read().await;
                    let sender = registry.lookup(&message.target);
                    let response = match sender {
                        Ok(mut tx) => {
                            tx.send(message).await
                                .map_err(|msg| Error::MessageSendFailed(msg.0.target))
                        },
                        Err(err)=> Err(err)
                    };
                    let _ = responder.send(response);
                },
                Request::Register(bus_name, responder) => {
                    let mut registry = self.registry.write().await;
                    let result = registry.register(bus_name);
                    let _ = responder.send(result);
                },
                Request::Unregister(bus_name) => {
                    let mut registry = self.registry.write().await;
                    registry.unregister(&bus_name);
                },
                Request::Lookup(bus_name, responder) => {
                    let registry = self.registry.read().await;
                    let result = registry.lookup(&bus_name);
                    let _ = responder.send(result);
                },
            }
        }
    }

    async fn next_message(&mut self) -> Option<Request> {
        self.incoming.next().await
    }
}


/// Handle to a bus.
#[derive(Clone)]
pub struct Handle {
    bus: RawHandle,
}

impl Handle {
    /// Register a new connection on the bus.
    pub async fn register<N>(&mut self, name: N) -> Result<Connection, Error>
        where N: TryInto<BusName, Error=Error>
    {
        let bus_name = name.try_into()?;

        let (responder, response) = response_channel();

        self.bus
            .send(Request::Register(bus_name.clone(), responder))
            .map_err(|_| Error::BusSendFailed)?;

        response.await
            .map_err(|_| Error::AddressRecvFailed)?
            .map(|rx| {
                Connection::new(bus_name, self.clone(), rx)
            })
    }

    /// Unregisters the bus connection with the given name.
    pub fn unregister(&self, name: BusName) -> Result<(), Error> {
        self.bus
            .send(Request::Unregister(name.clone()))
            .map_err(|_| Error::BusSendFailed)
    }

    /// Look up a bus address by name.
    pub async fn lookup<N>(&self, name: N) -> Result<Addr, Error>
        where N: TryInto<BusName, Error=Error>
    {
        let name = name.try_into()?;

        let (responder, response) = response_channel();

        self.bus
            .send(Request::Lookup(name.clone(), responder))
            .map_err(|_| Error::BusSendFailed)?;

        response.await
            .map_err(|_| Error::BusResponseFailed)?
            .map(|tx| Addr::new(name, tx))
    }

    /// Sends a message to be handled by the bus.
    pub async fn send(&self, msg: Message) -> Result<(), Error> {
        let (responder, response) = response_channel();

        self.bus
            .send(Request::Message(msg, responder))
            .map_err(|_| Error::BusSendFailed)?;

        response.await
            .map_err(|_| Error::BusResponseFailed)?
    }
}


// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn register_on_bus() {
        let mut handle = Bus::spawn();

        let client1= handle.register("test.client.1").await;
        assert_eq!(client1.is_ok(), true);

        let client2= handle.register("test.client.1").await;
        assert_eq!(client2.unwrap_err(), Error::AddressInUse);

        drop(client1);
        let addr2 = handle.register("test.client.1").await;
        assert_eq!(addr2.is_ok(), true);
    }

    #[tokio::test]
    async fn send_direct_message() {
        let mut handle = Bus::spawn();

        let client1 = handle
            .register("test.client.1").await
            .expect("Did not receive bus address");

        let result = client1.send("test.client.2", "hello").await;
        assert_eq!(
            result.unwrap_err(),
            Error::TargetAddressNotFound,
            "Sending to a unknown bus name should error"
        );

        let mut client2 = handle
            .register("test.client.2").await
            .expect("Did not receive bus address");
        client1.send("test.client.2", "hello").await.unwrap();

        let msg = client2.next_message().await;
        assert_eq!(
            msg.is_some(),
            true,
            "Sent message should show up on other client"
        );
    }
}
