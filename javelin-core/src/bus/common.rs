use {
    std::{
        convert::TryFrom,
        fmt::{self, Display},
    },
    tokio::sync::mpsc,
    super::{message::Message, Error},
};


pub(super) type BusSender = mpsc::Sender<Message>;
pub(super) type BusReceiver = mpsc::Receiver<Message>;
pub(super) fn bus_channel() -> (BusSender, BusReceiver) {
    mpsc::channel(16)
}


pub type EventId = String;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Event {
    pub origin: BusName,
    pub id: EventId,
}

impl Event {
    pub(super) fn new(origin: BusName, id: String) -> Self {
        Self { origin, id }
    }
}


#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct BusName(String);

impl Display for BusName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.0)
    }
}

impl TryFrom<String> for BusName {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        // TODO: validate bus name
        Ok(Self(value.to_string()))
    }
}

impl TryFrom<&str> for BusName {
    type Error = <Self as TryFrom<String>>::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(value.to_string())
    }
}
