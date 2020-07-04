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
        error::Error,
        common::{BusName, Request, response_channel},
        message::Message,
        registry::Registry,
        connection::{Connection, Addr},
    },
};


type Incoming = mpsc::UnboundedReceiver<Request<Message>>;
type RawHandle = mpsc::UnboundedSender<Request<Message>>;


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
    pub fn get_handle(&mut self) -> Handle {
        Handle { bus: self.handle.clone() }
    }

    async fn next_message(&mut self) -> Option<Request<Message>> {
        self.incoming.next().await
    }

    /// Start listening to bus messages.
    pub async fn listen(mut self) {
        while let Some(request) = self.next_message().await {
            match request {
                Request::WithoutResponse(_msg) => {
                    todo!();
                },
                Request::WithResponse(_msg, _responder) => {
                    todo!();
                },
                Request::Register(bus_name, responder) => {
                    let mut registry = self.registry.write().await;
                    let result = registry.register(bus_name);
                    // TODO: handle error
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
}


/// Handle to a bus.
#[derive(Clone)]
pub struct Handle {
    bus: RawHandle,
}

impl Handle {
    /// Register a new connection on the bus.
    pub async fn register<N>(&mut self, name: N) -> Result<Connection, Error>
        where N: TryInto<BusName>
    {
        let bus_name = name
            .try_into()
            .map_err(|_| Error::InvalidBusName)?;

        let (responder, response) = response_channel();

        // TODO: handle error
        let _ = self.bus.send(Request::Register(bus_name.clone(), responder));

        let rx = response.await
            .map_err(|_| Error::AddressRecvFailed)?;

        Ok(Connection::new(bus_name, self.clone(), rx?))
    }

    /// Unregisters the bus connection with the given name.
    pub fn unregister(&self, name: BusName) -> Result<(), Error> {
        if let Err(_) = self.bus.send(Request::Unregister(name)) {
            log::error!("Failed to unregister bus connection");
        }
        Ok(())
    }

    pub async fn lookup(&self, name: BusName) -> Result<Addr, Error> {
        let (responder, response) = response_channel();

        let _ = self.bus.send(Request::Lookup(name, responder));
        let tx = response.await
            .map_err(|_| Error::AddressRecvFailed)?;

        let addr = Addr::new(tx?);
        Ok(addr)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn spawn_bus() -> Handle {
        let mut bus = Bus::new();
        let handle = bus.get_handle();
        tokio::spawn(bus.listen());
        handle
    }

    #[tokio::test]
    async fn register_on_bus() {
        let mut handle = spawn_bus();

        let client1= handle.register("test.client.1").await;
        assert_eq!(client1.is_ok(), true);

        let client2= handle.register("test.client.1").await;
        match client2 {
            Err(Error::AddressInUse) => (),
            _ => panic!("Did not receive expected error")
        }

        drop(client1);
        let addr2 = handle.register("test.client.1").await;
        assert_eq!(addr2.is_ok(), true);
    }

    #[tokio::test]
    async fn send_direct_message() {
        let mut handle = spawn_bus();

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
