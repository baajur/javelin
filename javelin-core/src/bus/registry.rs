use {
    std::collections::{HashMap, hash_map::Entry},
    super::{
        common::{
            BusName, BusSender, BusReceiver, bus_channel,
            Event
        },
        Error
    },
};


#[derive(Default)]
pub(super) struct Registry {
    members: HashMap<BusName, BusSender>,
    events: HashMap<Event, Vec<BusSender>>,
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

    pub fn register_event(&mut self, event: Event) -> Result<(), Error> {
        match self.events.entry(event) {
            Entry::Occupied(_) => Err(Error::EventAlreadyRegistred),
            Entry::Vacant(map) => {
                map.insert(Vec::new());
                Ok(())
            }
        }
    }

    pub fn subscribe(&mut self, bus_name:  &BusName, event: &Event) -> Result<(), Error> {
        let sender = self.lookup(bus_name)?;
        match self.events.get_mut(event) {
            None => Err(Error::EventNotFound),
            Some(entry) => {
                entry.push(sender);
                Ok(())
            },
        }
    }

    pub fn listeners(&mut self, event: &Event) -> Option<&mut Vec<BusSender>> {
        self.events.get_mut(event)
    }
}
