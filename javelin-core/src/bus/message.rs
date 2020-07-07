use {
    serde::{Serialize, Deserialize},
    bytes::Bytes,
    super::common::BusName,
};


pub type MessagePayload = Bytes;

#[derive(Debug, Clone)]
pub struct Message {
    origin: Option<BusName>,
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
}
