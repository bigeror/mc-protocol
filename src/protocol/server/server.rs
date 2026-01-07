use std::{collections::HashMap, sync::{Arc, LazyLock}};
use tokio::sync::mpsc::{self, UnboundedSender};


pub struct Server {
    // uuid, username
    pub players: HashMap<(Arc<str>, Arc<str>), UnboundedSender<Vec<u8>>>,
}

pub enum Data {
    Packet {data: Vec<u8>, filter: Option<(Arc<str>, Arc<str>)>},
    AddPlayer {player: (Arc<str>, Arc<str>), sender: UnboundedSender<Vec<u8>>},
    RemovePlayer {player: (Arc<str>, Arc<str>)}
}

impl Server {
    pub fn new() -> Self { Self {
        players: HashMap::new()
    }}
    pub fn setup_sender(mut self) -> UnboundedSender<Data> {
        let (writer, mut reader) = mpsc::unbounded_channel();
        tokio::spawn(async move {loop {
            let data = match reader.recv().await {
                Some(val) => val,
                None => panic!("Server reader was closed!")
            };

            match data {
                Data::Packet { data, filter } => {
                    self.players.iter().for_each(move |((uuid, username), writer)| {
                        if let Some(filter_ptr) = filter.clone() {
                            if (uuid, username) == (&filter_ptr.0, &filter_ptr.1) {return}
                        }
                        _ = writer.send(data.clone());
                    });
                },
                Data::AddPlayer { player, sender } => {
                    self.players.insert(player, sender);
                },
                Data::RemovePlayer { player } => {
                    _ = self.players.remove(&player);
                },
            }
        }});
        writer
    }
}

pub static SERVER: LazyLock<UnboundedSender<Data>> = LazyLock::new(|| Server::new().setup_sender());
