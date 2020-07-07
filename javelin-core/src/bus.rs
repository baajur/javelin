// TODO: allowed to reduce noise, remove later
// #![allow(dead_code)]
// #![allow(unused_imports)]


mod error;
mod common;
mod message;
mod registry;
mod connection;


use {
    std::{convert::TryInto, sync::Arc},
    tokio::{
        sync::{mpsc, RwLock},
        stream::StreamExt,
    },
    self::{
        common::{BusName, Request, response_channel, Event},
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
    registry: Arc<RwLock<Registry>>,
    incoming: Incoming,
    handle: RawHandle,
}

impl Bus {
    pub fn new() -> Self {
        let registry = Arc::new(RwLock::new(Registry::new()));
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
                    let sender = registry.lookup(message.target.as_ref().unwrap());
                    let response = match sender {
                        Ok(mut tx) => {
                            tx.send(message).await
                                .map_err(|msg| Error::MessageSendFailed(msg.0.target.unwrap()))
                        },
                        Err(err)=> Err(err)
                    };
                    let _ = responder.send(response);
                },
                Request::Broadcast(event, message) => {
                    let mut registry = self.registry.write().await;
                    if let Some(event_listeners) = registry.listeners(&event) {
                        for event_listener in event_listeners {
                            let _ = event_listener.send(message.clone()).await;
                        }
                    }
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
                Request::RegisterEvent(event, responder) => {
                    let mut registry = self.registry.write().await;
                    let result = registry.register_event(event);
                    let _ = responder.send(result);
                },
                Request::Subscribe(bus_name, event, responder) => {
                    let mut registry = self.registry.write().await;
                    let result = registry.subscribe(&bus_name, &event);
                    let _ = responder.send(result);
                }
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

    pub async fn register_event(&self, event: Event) -> Result<(), Error> {
        let (responder, response) = response_channel();

        self.bus
            .send(Request::RegisterEvent(event, responder))
            .map_err(|_| Error::BusSendFailed)?;

        response.await
            .map_err(|_| Error::BusResponseFailed)?
    }

    pub async fn subscribe(&self, bus_name: BusName, event: Event) -> Result<(), Error> {
        let (responder, response) = response_channel();

        self.bus
            .send(Request::Subscribe(bus_name, event, responder))
            .map_err(|_| Error::BusSendFailed)?;

        response.await
            .map_err(|_| Error::BusResponseFailed)?
    }

    pub async fn broadcast(&self, event: Event, message: Message) -> Result<(), Error> {
        self.bus
            .send(Request::Broadcast(event, message))
            .map_err(|_| Error::BusSendFailed)
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

    #[tokio::test]
    async fn message_subscription_and_broadcast() {
        let mut handle = Bus::spawn();

        let mut client1 = handle
            .register("test.client.1").await
            .expect("Did not receive bus address");

        let mut client2 = handle
            .register("test.client.2").await
            .expect("Did not receive bus address");

        let client3 = handle
            .register("test.client.3").await
            .expect("Did not receive bus address");

        let _ = client3.register_event("hello").await;
        let _ = client1.subscribe("test.client.3", "hello").await;
        let _ = client2.subscribe("test.client.3", "hello").await;
        let _ = client3.broadcast("hello", "Hello There!").await;

        let msg1 = client1.next_message().await;
        let msg2 = client2.next_message().await;

        assert_eq!(msg1.is_some(), true);
        assert_eq!(msg2.is_some(), true);
    }
}
