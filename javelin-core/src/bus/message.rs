use {
    serde::{Serialize, Deserialize},
    bytes::Bytes,
    super::{
        common::BusName,
        error::Error,
    },
};


#[derive(Debug, Clone, PartialEq)]
pub enum MessagePayload {
    Ping,
    Bytes(Bytes),
}

impl From<Bytes> for MessagePayload {
    fn from(val: Bytes) -> Self {
        Self::Bytes(val)
    }
}

impl From<Vec<u8>> for MessagePayload {
    fn from(val: Vec<u8>) -> Self {
        Self::from(Bytes::from(val))
    }
}


#[derive(Debug, Clone)]
pub struct Message {
    pub origin: Option<BusName>,
    pub target: Option<BusName>,
    pub payload: MessagePayload,
}

impl Message {
    pub fn new<'de, P>(target: Option<BusName>, payload: P) -> Self
        where P: Serialize + Deserialize<'de>
    {
        let payload = bincode::serialize(&payload).unwrap().into();
        Self { target, payload, origin: None }
    }

    pub fn new_raw<P>(target: Option<BusName>, payload: P) -> Self
        where P: Into<MessagePayload>
    {
        Self { target, payload: payload.into(), origin: None }
    }

    pub fn ping() -> Self {
        Self::new(None, ())
    }

    pub fn unpack<'de, T>(&'de self) -> Result<T, Error>
        where T: Deserialize<'de>
    {
        match &self.payload {
            MessagePayload::Bytes(bytes) => {
                bincode::deserialize(&bytes)
                    .map_err(|_| Error::MessageUnpackFailed)
            },
            _ => Err(Error::MessageUnpackFailed)
        }
    }
}
