use std::{collections::HashMap, sync::{Arc, LazyLock}};
use crab_nbt::nbt;
use parking_lot::Mutex;
use tokio::sync::mpsc::{self, UnboundedSender};

use crate::protocol::{datatypes::{Vector2, Vector3}, play::clientbound::CLIENT_BOUND_PACKETS};

#[derive(Debug)]
pub struct Server {
    // uuid, username
    pub players: HashMap<(Arc<str>, Arc<str>), (UnboundedSender<Vec<u8>>, i32)>,
    pub positions: HashMap<i32, (Vector3<f64>, Vector2<f32>, bool)>,
    pub eids: Vec<i32>,
}

#[derive(Debug)]
pub enum Data {
    Packet {data: Vec<u8>, filter: Option<(Arc<str>, Arc<str>)>},
    AddPlayer {
        player: (Arc<str>, Arc<str>),
        sender: UnboundedSender<Vec<u8>>,
        eid: i32,
        position: Vector3<f64>,
        rotation: Vector2<f32>,
    },
    RemovePlayer {player: (Arc<str>, Arc<str>) },
    UpdatePosition {eid: i32, position: Vector3<f64>, rotation: Vector2<f32>, on_ground: bool}
}

impl Server {
    pub fn new() -> Self { Self {
        players: HashMap::new(),
        positions: HashMap::new(),
        eids: Vec::new(),
    }}
    pub fn setup_sender(self) -> ServerStatic {
        let (sender, mut reader) = mpsc::unbounded_channel();
        let mutex = Arc::from(Mutex::new(self));
        let mutex_clone = mutex.clone();
        let sender_clone = sender.clone();

        tokio::spawn(async move {loop {
            let data = match reader.recv().await {
                Some(val) => val,
                None => panic!("Server reader was closed!")
            };
            Self::handle_data(data, mutex_clone.clone(), sender_clone.clone());
        }});
        ServerStatic { sender, mutex }
    }
    pub fn get_push_eid(&mut self) -> i32 {
        let eid = match self.eids.iter().max()
            {Some(val) => *val + 1, None => 0};
        self.eids.push(eid);
        eid
    }
    pub fn handle_data(data: Data, mutex: Arc<Mutex<Server>>, writer: UnboundedSender<Data>) {
        let mut lock = mutex.lock();

        match data {
            Data::Packet { data, filter } => {
                for ((uuid, username), (writer, _))
                in lock.players.iter() {
                    if let Some(filter_ptr) = filter.clone() {
                        if (uuid, username) == (&filter_ptr.0, &filter_ptr.1) {continue}
                    }
                    _ = writer.send(data.clone());
                };
            },

            Data::AddPlayer {
                player,
                sender,
                eid,
                position,
                rotation
            } => {
                lock.players.insert(player.clone(), (sender, eid));
                lock.positions.insert(eid, (position, rotation, true));

                let mut players = Vec::new();
                for player in lock.players.iter() {
                    players.push((player.0.0.clone(), player.0.1.clone(), player.1.1));
                }

                let player_name: &str = &player.1.clone();
                let player_packet = [
                    (CLIENT_BOUND_PACKETS.send_system_message)(nbt!("", {
                        "text": "",
                        "extra": [
                            {"text": "[", "color": "gray"},
                            {"text": "+", "color": "green"},
                            {"text": "] ", "color": "gray"},
                            {"text": player_name}
                        ]
                    }).write_unnamed().to_vec(), false).unwrap(),
                    (CLIENT_BOUND_PACKETS.player_info_update)(players).unwrap(),
                    (CLIENT_BOUND_PACKETS.summon_entity)(
                        eid, player.0.clone(), 149, position, rotation, 0, Vector3 { x: 0.0, y: 0.0, z: 0.0 }
                    ).unwrap(),
                ].concat();

                println!("[+] {} [{}]", player.1, player.0);
                _ = writer.send(Data::Packet { data: player_packet, filter: Some(player) });
            },

            Data::RemovePlayer { player } => {
                let Some(eid) = lock.players.remove(&player) else {return};
                let eid = eid.1;

                let player_name: &str = &player.1.clone();
                let player_packet = [
                    (CLIENT_BOUND_PACKETS.send_system_message)(nbt!("", {
                        "text": "",
                        "extra": [
                            {"text": "[", "color": "gray"},
                            {"text": "-", "color": "red"},
                            {"text": "] ", "color": "gray"},
                            {"text": player_name}
                        ]
                    }).write_unnamed().to_vec(), false).unwrap(),
                    (CLIENT_BOUND_PACKETS.player_info_remove)(player.0.clone()).unwrap(),
                    (CLIENT_BOUND_PACKETS.remove_entity)(vec![eid]).unwrap(),
                ].concat();

                println!("[-] {} [{}]", player.1, player.0);
                _ = writer.send(Data::Packet { data: player_packet, filter: Some(player.clone()) });

                lock.positions.remove(&eid);
                lock.eids.retain(|val| val != &eid);
            },

            Data::UpdatePosition {
                eid,
                position,
                rotation,
                on_ground
            } => {
                todo!()
            },
        }
    }
}

pub struct ServerStatic {
    pub sender: UnboundedSender<Data>, 
    pub mutex: Arc<Mutex<Server>>
}

pub static SERVER: LazyLock<ServerStatic> = LazyLock::new(|| Server::new().setup_sender());
