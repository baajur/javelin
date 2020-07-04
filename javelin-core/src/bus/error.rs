use thiserror::Error;


#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("Target address could not be found")]
    TargetAddressNotFound,

    #[error("Requested bus address is already in use")]
    AddressInUse,

    #[error("No registered event found with this name")]
    EventNotFound,

    #[error("Bus name is not valid")]
    InvalidBusName,

    #[error("Failed to receive address from bus")]
    AddressRecvFailed,

    #[error("Failed to create message")]
    MessageCreationFailed,

    #[error("Tried to send request to closed bus")]
    BusClosed,
}
