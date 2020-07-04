use bytes::Bytes;


pub type MessagePayload = Bytes;

#[derive(Debug, PartialEq, Eq)]
pub struct Message {
    pub payload: MessagePayload,
}

impl Message {
    pub fn new<P>(payload: P) -> Self
        where P: Into<Bytes>
    {
        Self {
            payload: payload.into()
        }
    }
}
