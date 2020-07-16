mod error;
mod common;
mod message;
mod registry;
mod connection;


use {
    std::{convert::TryInto, sync::Arc},
    tokio::sync::RwLock,
    self::{
        common::{BusName, Event},
        registry::Registry,
    },
};

pub use self::{
    error::Error,
    message::Message,
    connection::{Connection, Addr},
};


#[derive(Clone, Default)]
pub struct Bus {
    registry: Arc<RwLock<Registry>>,
}

impl Bus {
    pub fn new() -> Self {
        Self::default()
     }

    /// Register a new connection on the bus.
    pub async fn register<N>(&mut self, name: N) -> Result<Connection, Error>
        where N: TryInto<BusName, Error=Error>
    {
        let name = name.try_into()?;
        let mut registry = self.registry.write().await;

        if let Ok(mut sender) = registry.lookup(&name) {
            match sender.send(Message::ping()).await {
                Ok(_) => return Err(Error::AddressInUse),
                Err(_) => registry.unregister(&name)
            }
        }

        registry
            .register(name.clone())
            .map(|rx| {
                Connection::new(name, self.clone(), rx)
            })
    }

    /// Look up a bus address by name.
    pub async fn lookup<N>(&self, name: N) -> Result<Addr, Error>
        where N: TryInto<BusName, Error=Error>
    {
        let name = name.try_into()?;
        let registry = self.registry.read().await;
        registry.lookup(&name)
            .map(|tx| Addr::new(name, tx))
    }

    /// Sends a message to be handled by the bus.
    pub async fn send(&self, message: Message) -> Result<(), Error> {
        let registry = self.registry.read().await;
        let mut sender = registry.lookup(message.target.as_ref().unwrap())?;
        sender.send(message).await
            .map_err(|msg| Error::MessageSendFailed(msg.0.target.unwrap()))
    }

    pub async fn subscribe(&self, bus_name: BusName, event: Event) -> Result<(), Error> {
        let mut registry = self.registry.write().await;
        registry.subscribe(&bus_name, event)
    }

    pub async fn broadcast(&self, event: Event, message: Message) -> Result<(), Error> {
        let mut registry = self.registry.write().await;

        if let Some(event_listeners) = registry.listeners(&event) {
            for event_listener in event_listeners {
                let _ = event_listener.send(message.clone()).await;
            }
        }

        Ok(())
    }
}


// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn register_on_bus() {
        let mut handle = Bus::new();

        let client1= handle.register("test.client.1").await;
        assert_eq!(client1.is_ok(), true, "Failed to register new client");

        let client2= handle.register("test.client.1").await;
        assert_eq!(client2.is_ok(), false, "Should not allow two clients with the same name");

        drop(client1);
        let addr2 = handle.register("test.client.1").await;
        assert_eq!(addr2.is_ok(), true, "Failed to register new client after dropping old one");
    }

    #[tokio::test]
    async fn send_direct_message() {
        let mut handle = Bus::new();

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

        let msg = client2.next_message().await
            .expect("Failed to receive message").unwrap();
        let msg: String = msg.unpack()
            .expect("Failed to unpack received message");

        assert_eq!(msg, "hello", "Sent message should show up on other client");
    }

    #[tokio::test]
    async fn message_subscription_and_broadcast() {
        let mut handle = Bus::new();

        let mut client1 = handle
            .register("test.client.1").await
            .expect("Did not receive bus address");

        let mut client2 = handle
            .register("test.client.2").await
            .expect("Did not receive bus address");

        let client3 = handle
            .register("test.client.3").await
            .expect("Did not receive bus address");

        let _ = client1.subscribe("test.client.3", "hello").await;
        let _ = client2.subscribe("test.client.3", "hello").await;
        let _ = client3.broadcast("hello", "Hello There!").await;

        let msg1 = client1.next_message().await;
        let msg2 = client2.next_message().await;

        assert_eq!(msg1.is_ok(), true);
        assert_eq!(msg2.is_ok(), true);
    }
}
