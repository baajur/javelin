use {
    serde::{Serialize, Deserialize},
    bytes::Bytes,
    super::{
        common::{BusName, Responder},
        error::Error,
    },
};


pub type MessagePayload = Bytes;

#[derive(Debug)]
pub struct Message {
    origin: Option<Responder<Message>>,
    pub target: BusName,
    pub payload: MessagePayload,
}

impl Message {
    pub fn new<'de, P>(target: BusName, payload: P) -> Self
        where P: Serialize + Deserialize<'de>
    {
        let payload = bincode::serialize(&payload).unwrap().into();
        Self { target, payload, origin: None }
    }

    pub fn new_raw<P>(target: BusName, payload: P) -> Self
        where P: Into<MessagePayload>
    {
        Self { target, payload: payload.into(), origin: None }
    }

    pub async fn respond(self, message: Message) -> Result<(), Error> {
        if let Some(responder) = self.origin {
            responder.send(message);
        }

        Ok(())
    }
}
