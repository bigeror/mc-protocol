use std::{collections::HashMap, sync::{Arc, LazyLock, Mutex}};

use futures::{SinkExt, channel::mpsc::UnboundedSender};

pub struct Server {
    // uuid, username
    pub players: HashMap<(Arc<str>, Arc<str>), UnboundedSender<Vec<u8>>>,
}
impl Server {
    pub fn new() -> Self {Self {
        players: HashMap::new()
    }}
    pub fn send_to_players(&mut self, packet: Vec<u8>, filter: Option<(&Arc<str>, &Arc<str>)>) {
        self.players.iter().for_each(|((uuid, username), mut writer)| {
            if Some((uuid, username)) == filter {return}
            _ = writer.send(packet.clone());
        });
    }
}

pub static SERVER: LazyLock<Mutex<Server>> = LazyLock::new(|| Mutex::new(Server::new()));
