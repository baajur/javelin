use {
    std::{
        convert::TryInto,
        fmt::{self, Debug},
    },
    serde::{Serialize, Deserialize},
    super::{
        common::{BusName, BusSender, BusReceiver, Event, EventId},
        message::{Message, MessagePayload},
        Error, Bus,
    },
};


/// Represents a connection to the bus system.
pub struct Connection {
    name: BusName,
    handle: Bus,
    rx: BusReceiver,
}

impl Connection {
    pub(super) fn new(name: BusName, handle: Bus, rx: BusReceiver) -> Self {
        Self { name, handle, rx }
    }

    pub async  fn send<'de, N, M>(&self, name: N, msg: M) -> Result<(), Error>
        where N: TryInto<BusName, Error=Error>,
              M: Serialize + Deserialize<'de>,
    {
        let name = name.try_into()?;
        let message = Message::new(Some(name), msg);
        self.handle.send(message).await
    }

    pub async fn send_raw<N, M>(&self, name: N, msg: M) -> Result<(), Error>
        where N: TryInto<BusName, Error=Error>,
              M: Into<MessagePayload>
    {
        let name = name.try_into()?;
        let message = Message::new_raw(Some(name), msg);
        self.handle.send(message).await
    }

    pub async fn subscribe<N, E>(&self, name: N, id: E) -> Result<(), Error>
        where N: TryInto<BusName, Error=Error>,
              E: Into<EventId>
    {
        let name = name.try_into()?;
        let event_id = id.into();
        let event = Event::new(name, event_id.clone());
        self.handle.subscribe(self.name.clone(), event).await
    }

    pub async fn broadcast<'de, E, M>(&self, id: E, msg: M) -> Result<(), Error>
        where E: Into<EventId>,
              M: Serialize + Deserialize<'de>
    {
        let message = Message::new(None, msg);
        let event = Event::new(self.name.clone(), id.into());
        self.handle.broadcast(event, message).await
    }

    pub async fn next_message(&mut self) -> Result<Option<Message>, Error> {
        loop {
            let msg = self.rx.recv().await;
            match msg {
                None => return Ok(None),
                Some(m) if m.payload == MessagePayload::Ping => continue,
                Some(m) => return Ok(Some(m))
            }
        }
    }
}

impl Debug for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Connection")
            .field("name", &self.name)
            .finish()
    }
}


/// Direct connection handle to a bus member.
#[derive(Debug)]
pub struct Addr {
    /// Name of the address target
    name: BusName,
    tx: BusSender,
}

impl Addr {
    pub(super) fn new(name: BusName, tx: BusSender) -> Self {
        Self { name, tx }
    }

    pub async fn send<M>(&mut self, msg: M) -> Result<(), Error>
        where M: Into<MessagePayload>
    {
        let message = Message::new_raw(Some(self.name.clone()), msg);

        // TODO: handle errors
        let _ = self.tx.send(message).await;

        Ok(())
    }
}
