use {
    thiserror::Error,
    super::common::BusName,
};


#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("Target address could not be found")]
    TargetAddressNotFound,

    #[error("Requested bus address is already in use")]
    AddressInUse,

    #[error("No registered event found with this name")]
    EventNotFound,

    #[error("Event is already registred")]
    EventAlreadyRegistred,

    #[error("Bus name is not valid")]
    InvalidBusName,

    #[error("Failed to receive address from bus")]
    AddressRecvFailed,

    #[error("Failed to create message")]
    MessageCreationFailed,

    #[error("Failed to send message to {0}")]
    MessageSendFailed(BusName),

    #[error("Tried to send request to closed bus")]
    BusClosed,

    #[error("Bus failed to return response")]
    BusResponseFailed,

    #[error("Failed to send message to bus")]
    BusSendFailed,
}
