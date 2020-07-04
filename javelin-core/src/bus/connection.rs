use {
    std::convert::TryInto,
    super::{
        common::{BusName, BusSender, BusReceiver},
        message::{Message, MessagePayload},
        Error,
        Handle,
    },
};


pub struct Connection {
    name: BusName,
    handle: Handle,
    rx: BusReceiver,
}

impl Connection {
    pub(super) fn new(name: BusName, handle: Handle, rx: BusReceiver) -> Self {
        Self { name, handle, rx }
    }

    /// Lookup a bus name for direct connection
    pub async fn send<N, M>(&self, name: N, msg: M) -> Result<(), Error>
        where N: TryInto<BusName, Error=Error>,
              M: Into<MessagePayload>
    {
        let name = name.try_into()?;
        let mut addr = self.handle.lookup(name).await?;
        addr.send(msg).await?;
        Ok(())
    }

    pub async fn next_message(&mut self) -> Option<Message> {
        self.rx.recv().await
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.handle.unregister(self.name.clone()).unwrap();
    }
}


#[derive(Debug)]
pub struct Addr {
    tx: BusSender,
}

impl Addr {
    pub(super) fn new(tx: BusSender) -> Self {
        Self { tx }
    }

    pub async fn send<M>(&mut self, msg: M) -> Result<(), Error>
        where M: Into<MessagePayload>
    {
        let message = Message::new(msg);

        // TODO: handle errors
        let _ = self.tx.send(message).await;

        Ok(())
    }
}
