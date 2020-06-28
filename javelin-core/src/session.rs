use {
    std::convert::TryFrom,
    bytes::Bytes,
    tokio::sync::{mpsc, oneshot},
    chrono::prelude::{DateTime, Utc},
    anyhow::Result,
    javelin_types::{Packet, PacketType, Metadata},
};


pub type TriggerReturnValue<V> = oneshot::Sender<V>;
pub type TriggerPayload = (String, TriggerReturnValue<Sender>);
pub type Trigger = mpsc::UnboundedSender<TriggerPayload>;
pub type OnTrigger = mpsc::UnboundedReceiver<TriggerPayload>;

pub fn trigger_channel() -> (Trigger, OnTrigger) {
    mpsc::unbounded_channel()
}


#[derive(Clone)]
pub enum Message {
    Packet(Packet),
    Disconnect,
}

pub type Sender = mpsc::UnboundedSender<Message>;
pub type Receiver = mpsc::UnboundedReceiver<Message>;

pub fn channel() -> (Sender, Receiver) {
    mpsc::unbounded_channel()
}


pub struct Session {
    pub watchers: Vec<Sender>,
    pub metadata: Option<Metadata>,
    pub video_seq_header: Option<Bytes>,
    pub audio_seq_header: Option<Bytes>,
    pub publish_start: DateTime<Utc>,
}

impl Session {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            watchers: Vec::new(),
            metadata: None,
            video_seq_header: None,
            audio_seq_header: None,
            publish_start: Utc::now(),
        }
    }

    pub fn add_watcher(&mut self, sender: Sender) {
        self.watchers.push(sender);
    }

    pub fn set_cache(&mut self, packet: Packet) -> Result<()> {
        match packet.kind {
            PacketType::Meta if self.metadata.is_none() => {
                let metadata = Metadata::try_from(packet).unwrap();
                self.metadata = Some(metadata);
            },
            PacketType::Video if self.video_seq_header.is_none() => {
                self.video_seq_header = Some(packet.payload);
            },
            PacketType::Audio if self.audio_seq_header.is_none() => {
                self.audio_seq_header = Some(packet.payload);
            }
            _ => ()
        }

        Ok(())
    }

    pub fn send_to_watchers(&mut self, message: Message) {
        self.watchers
            .retain(|watcher| {
                watcher.send(message.clone())
                    .map_err(|e| log::error!("Failed to send to watcher: {}", e))
                    .is_ok()
            })
    }

}
