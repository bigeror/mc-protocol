use std::{collections::HashMap, sync::{Arc, LazyLock}};
use crab_nbt::nbt;
use tokio::sync::{Mutex, mpsc::{self, UnboundedSender}};

use crate::protocol::{datatypes::{Display, PlayerKey, ServerPlayer, Vector2, Vector3, SendPacket}, play::clientbound::CLIENT_BOUND_PACKETS};
use SendPacket::SendPacket as Packet;

#[derive(Debug)]
pub struct Server {
    // uuid, username
    pub players: HashMap<PlayerKey, ServerPlayer>,
    pub eids: Vec<i32>,
}

#[derive(Debug)]
pub enum Data {
    Packet {data: Vec<u8>, filter: Option<PlayerKey>},
    AddPlayer {
        player: PlayerKey,
        sender: UnboundedSender<SendPacket>,
        eid: i32,
        position: Vector3<f64>,
        rotation: Vector2<f32>,
    },
    RemovePlayer {player: PlayerKey },
    UpdatePosition {player_key: PlayerKey, position: Vector3<f64>, rotation: Vector2<f32>, on_ground: bool}
}

impl Server {
    pub fn new() -> Self { Self {
        players: HashMap::new(),
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
            Self::handle_data(data, mutex_clone.clone(), sender_clone.clone()).await;
        }});
        ServerStatic { sender, mutex }
    }

    pub fn get_push_eid(&mut self) -> i32 {
        let eid = match self.eids.iter().max()
            {Some(val) => *val + 1, None => 0};
        self.eids.push(eid);
        eid
    }

    pub async fn handle_data(data: Data, mutex: Arc<Mutex<Server>>, writer: UnboundedSender<Data>) {
        let mut lock = mutex.lock().await;

        match data {
            Data::Packet { data, filter } => {
                for (key, server_player) in lock.players.iter() {
                    let writer = server_player.sender.clone();
                    if let Some(filter_ptr) = filter.clone() {
                        if key == &filter_ptr {continue}
                    }
                    _ = writer.send(Packet(data.clone()));
                };
            },

            Data::AddPlayer {
                player,
                sender,
                eid,
                position,
                rotation
            } => {
                lock.players.insert(player.clone(),
                    ServerPlayer { sender, position, rotation, eid, on_ground: false });

                let mut players = Vec::new();
                for player in lock.players.iter() { players.push(
                    PlayerKey { uuid: player.0.uuid, username: player.0.username.clone(), eid: player.0.eid }
                ) }

                let player_name: &str = &player.username.clone();
                let player_uuid: &str = &player.uuid.display();
                let player_packet = [
                    (CLIENT_BOUND_PACKETS.send_system_message)(nbt!("", {
                        "text": "",
                        "extra": [
                            {"text": "[", "color": "gray"},
                            {"text": "+", "color": "green"},
                            {"text": "] ", "color": "gray"},
                            {"text": player_name, "hover_event": {"action": "show_text", "value": player_uuid}}
                        ]
                    }).write_unnamed().to_vec(), false).unwrap(),
                    (CLIENT_BOUND_PACKETS.player_info_update)(players).unwrap(),
                    (CLIENT_BOUND_PACKETS.summon_entity)(
                        eid, player.uuid, 149, position, rotation, 0, Vector3 { x: 0.0, y: 0.0, z: 0.0 }
                    ).unwrap(),
                ].concat();

                println!("[+] {} [{}]", player.username, player.uuid.display());
                _ = writer.send(Data::Packet { data: player_packet, filter: Some(player) });
            },

            Data::RemovePlayer { player } => {
                let Some(eid) = lock.players.remove(&player) else {return};
                let eid = eid.eid;

                let player_name: &str = &player.username.clone();
                let player_uuid: &str = &player.uuid.display();
                let player_packet = [
                    (CLIENT_BOUND_PACKETS.send_system_message)(nbt!("", {
                        "text": "",
                        "extra": [
                            {"text": "[", "color": "gray"},
                            {"text": "-", "color": "red"},
                            {"text": "] ", "color": "gray"},
                            {"text": player_name, "hover_event": {"action": "show_text", "value": player_uuid}}
                        ]
                    }).write_unnamed().to_vec(), false).unwrap(),
                    (CLIENT_BOUND_PACKETS.player_info_remove)(player.uuid).unwrap(),
                    (CLIENT_BOUND_PACKETS.remove_entity)(vec![eid]).unwrap(),
                ].concat();

                println!("[-] {} [{}]", player.username, player.uuid.display());
                _ = writer.send(Data::Packet { data: player_packet, filter: Some(player.clone()) });

                lock.eids.retain(|val| val != &eid);
            },

            Data::UpdatePosition {
                player_key,
                position,
                rotation,
                on_ground
            } => {
                let Some(player) = lock.players.get_mut(&player_key) else {return};
                let delta = Vector3 {
                    x: position.x - player.position.x,
                    y: position.y - player.position.y,
                    z: position.z - player.position.z,
                };

                player.position = position;
                player.rotation = rotation;
                player.on_ground = on_ground;

                let response = [
                    (CLIENT_BOUND_PACKETS.update_position)( player.eid, delta, rotation, on_ground ).unwrap(),
                    (CLIENT_BOUND_PACKETS.set_head_rotation)( player.eid, rotation.x ).unwrap(),
                ].concat();

                _ = writer.send(Data::Packet { data: response, filter: Some(player_key) })
            },
        }
    }
}

pub struct ServerStatic {
    pub sender: UnboundedSender<Data>, 
    pub mutex: Arc<Mutex<Server>>
}

pub static SERVER: LazyLock<ServerStatic> = LazyLock::new(|| Server::new().setup_sender());
