use {
    std::convert::TryFrom,
    tokio::sync::{mpsc, oneshot},
    super::{message::Message, Error},
};


pub(super) type BusSender = mpsc::Sender<Message>;
pub(super) type BusReceiver = mpsc::Receiver<Message>;
pub(super) fn bus_channel() -> (BusSender, BusReceiver) {
    mpsc::channel(16)
}


/// Result of a response that has been sent back
pub(super) type Response<P> = Result<P, Error>;

/// Channel with which a response can be sent back
pub(super) type Responder<P> = oneshot::Sender<Response<P>>;
pub(super) type ResponseHandle<P> = oneshot::Receiver<Response<P>>;
pub(super) fn response_channel<P>() -> (Responder<P>, ResponseHandle<P>) {
    oneshot::channel()
}

pub(super) enum Request<M, R=M> {
    WithResponse(M, Responder<R>),
    WithoutResponse(M),
    Register(BusName, Responder<BusReceiver>),
    Unregister(BusName),
    Lookup(BusName, Responder<BusSender>),
}


#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct BusName(String);

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
