use {
    std::collections::{HashMap, hash_map::Entry},
    super::{
        common::{BusName, BusSender, BusReceiver, bus_channel},
        Error
    },
};


#[derive(Default)]
pub(super) struct Registry {
    members: HashMap<BusName, BusSender>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, name: BusName) -> Result<BusReceiver, Error> {
        match self.members.entry(name) {
            Entry::Occupied(_) => Err(Error::AddressInUse),
            Entry::Vacant(map) => {
                let (tx, rx) = bus_channel();
                map.insert(tx);
                Ok(rx)
            }
        }
    }

    pub fn unregister(&mut self, name: &BusName) {
        self.members.remove(name);
    }

    pub fn lookup(&self, name: &BusName) -> Result<BusSender, Error> {
        self.members
            .get(name)
            .cloned()
            .ok_or(Error::TargetAddressNotFound)
    }
}
