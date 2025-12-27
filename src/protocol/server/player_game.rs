use std::{collections::HashMap, sync::Arc, time::Duration};

use rand::{random, random_range};
use tokio::{sync::{mpsc::UnboundedSender, Mutex}, time::sleep};

use crate::{concat_buffer, datatypes::UUID, protocol::{datatypes::{Player, PlayerInput, Vector2, Vector3}, play::clientbound::CLIENT_BOUND_PACKETS}};

#[derive(Debug)]
pub struct Game {
    pub player: Player,
    pub writer: UnboundedSender<Vec<u8>>,
    entity_list: Vec<Arc<Mutex<Entity>>>,
    latest_entity_id: i32,
    pub stop: bool,
    pub input: PlayerInput,
    player_position: Vector2<f64>,
    player_xp: u16,
    player_health: u16,
}

#[allow(unused_assignments)]
#[allow(unused_mut)]
impl Game {
    pub fn new(player: Player, writer: UnboundedSender<Vec<u8>>) -> Self {
        Game {
            player: player, 
            writer,
            entity_list: Vec::new(),
            latest_entity_id: 1,
            stop: false,
            input: PlayerInput::empty(),
            player_position: Vector2 { x: 0.0, y: 0.0 },
            player_xp: 0,
            player_health: 100
        }
    }

    pub fn start_main_loop(this_raw: Arc<Mutex<Game>>) {
        tokio::spawn(async move {
            let mut counter = 0;
            let mut background: Vec<Arc<Mutex<Entity>>> = Vec::new();
            let mut enemies: Vec<Enemy> = Vec::new();
            let mut bullets: Vec<Bullet> = Vec::new();
            let mut shoot_buffer = false;

            {
                let mut this = this_raw.lock().await;
                let mut packet: Vec<u8> = Vec::new();
                let slot_data = concat_buffer!(unwrap: varint 1, varint 1008, varint 0, varint 0);

                for x in -10..11 {for y in -7..8 {
                    let (spawn_packet, entity) = this.summon(
                        Vector3 { x: 0.0, y: 129.65, z: 7.0 },
                        Vector2 { x: 0.0, y: 0.0 }, 70
                    );
                    background.push(entity.clone());
                    let entity_lock = entity.lock().await;
                    packet.extend([spawn_packet, (CLIENT_BOUND_PACKETS.add_entity_metadata)(
                        entity_lock.eid, vec![(23, 7, slot_data.clone()), (11, 33, concat_buffer!(unwrap: float x as f32 + 0.5, float y as f32 + 0.5, float 0.0))]
                    ).unwrap()].concat());
                }};

                _ = this.writer.send(packet);
            }

            loop {
                sleep(Duration::from_millis(50)).await;
                let mut this = this_raw.lock().await;
                if (*this).stop == true {return}

                let mut response = (CLIENT_BOUND_PACKETS.particle)(
                    true,
                    Vector3 {
                        x: this.player.rotation.x as f64,
                        y: this.player.rotation.y as f64 + 129.65,
                        z: 1.0
                    },
                    Vector3 {x: 0.0, y:0.0, z:0.0},
                    0.0, 1, 3, vec![]
                ).unwrap(); // particle :D

                let movement = Vector2 {y: this.input.forward as u32 as f64 - this.input.backward as u32 as f64, x: this.input.left as u32 as f64 - this.input.right as u32 as f64};
                this.player_position = Vector2 { x: this.player_position.x + movement.x * 0.15, y: this.player_position.y + movement.y * 0.15 };

                for entity_raw in background.clone() {
                    let mut entity = entity_raw.lock().await;
                    response.extend(entity.move_entity(
                        Vector3 {x: -this.player_position.x.rem_euclid(1.0), y: 129.65 -this.player_position.y.rem_euclid(1.0), z: 7.0 }, 
                        Vector2 { x: 0.0, y: 0.0 }))
                }

                if this.input.jump {shoot_buffer = true}
                if counter == 0 && shoot_buffer {
                    let (pack, entity) = this.summon(Vector3 {x:0.0, y:129.65, z:6.99}, Vector2 {x:0.0, y:0.0}, 70);
                    let entity_clone = entity.clone();
                    let mut entity_lock = entity_clone.lock().await;
                    response.extend([pack, (CLIENT_BOUND_PACKETS.add_entity_metadata)(entity_lock.eid, vec![
                            (23, 7, concat_buffer!(unwrap: varint 1, varint 1008, varint 0, varint 0)),
                            (11, 33, concat_buffer!(unwrap: float 0.0, float 0.0, float 0.0)),
                            (8, 1, concat_buffer!(unwrap: varint 1))
                    ]).unwrap()].concat());
                    bullets.push(Bullet { friendly: true, entity, position: this.player_position, direction: random_range(0.0..360.0) });
                    shoot_buffer = false
                };

                if counter == 0 && this.input.sneak {
                    let (pack, entity) = this.summon(Vector3 {x:0.0, y:129.65, z:6.99}, Vector2 {x:0.0, y:0.0}, 70);
                    let entity_clone = entity.clone();
                    let mut entity_lock = entity_clone.lock().await;
                    response.extend([pack, (CLIENT_BOUND_PACKETS.add_entity_metadata)(entity_lock.eid, vec![
                            (23, 7, concat_buffer!(unwrap: varint 1, varint 1008, varint 0, varint 0)),
                            (11, 33, concat_buffer!(unwrap: float 0.0, float 0.0, float 0.0)),
                            (8, 1, concat_buffer!(unwrap: varint 1))
                    ]).unwrap()].concat());
                    enemies.push(Enemy { entity, position: this.player_position, target_offset: Vector2 {x: 0.0, y: 0.0}, health: 0, tick: 0 });
                }

                let mut bullet_grid: HashMap<(i8, i8), Vec<usize>> = HashMap::new();
                let mut delete_bullet_indicies: Vec<usize> = Vec::new();
                let mut delete_enemy_indicies: Vec<usize> = Vec::new();

                for (index, value) in bullets.iter_mut().enumerate() {
                    let mut entity = value.entity.lock().await;
                    let new_pos_main = value.position + Vector2{x:value.direction.cos() as f64, y:value.direction.sin() as f64} .scale(1.0);
                    value.position = new_pos_main;
                    let new_pos = Vector3 { x: new_pos_main.x - this.player_position.x, y: new_pos_main.y - this.player_position.y + 129.65, z: 6.99 };

                    if new_pos.x.abs() >= 10.0 || (new_pos.y - 129.65).abs() >= 7.0 {
                        response.extend(this.kill(&value.entity, entity.eid));
                        delete_bullet_indicies.insert(0, index);
                        continue;
                    }

                    let grid_coordinates = ((new_pos_main.x - this.player_position.x).floor() as i8, (new_pos_main.x - this.player_position.x).floor() as i8);
                    if value.friendly {_ = bullet_grid.entry(grid_coordinates).and_modify(|val| val.push(index)).or_insert(vec![index])}
                    if !value.friendly && new_pos_main.length() < 0.5 {} // TODO: add player damage

                    response.extend(entity.move_entity(new_pos, Vector2 {x:0.0, y:0.0}));
                }

                for (index, value) in enemies.iter_mut().enumerate() {
                    let mut entity = value.entity.lock().await;
                    let grid_coordinates = Vector2 {x: (value.position.x - this.player_position.x).floor() as i8, y: (value.position.x - this.player_position.x).floor() as i8};
                    let mut is_damaged = false;
                    let new_pos = Vector3 {x: value.position.x - this.player_position.x, y: value.position.y - this.player_position.y + 129.65, z: 6.98};
                    response.extend(entity.move_entity(new_pos, Vector2 {x: 0.0, y: 0.0}));

                    for i in 0..9 {
                        let coord = Vector2 {x: (i as f64 / 3.0).floor() as i8 - 1, y: i % 3 as i8 - 1};
                        match bullet_grid.get(&((grid_coordinates + coord).x as i8, (grid_coordinates + coord).y as i8)) {
                            None => continue,
                            Some(indicies) => {for index in indicies {
                                let bullet_pos = bullets[*index].position;
                                if (bullet_pos - value.position).length() < 0.3 {is_damaged = true} // TODO: add entity damage
                            }}
                        }
                    }

                    if is_damaged {
                        response.extend(this.kill(&value.entity, entity.eid));
                        delete_enemy_indicies.insert(0, index);
                        continue;
                    }
                }

                for index in delete_bullet_indicies {_ = bullets.remove(index)}
                for index in delete_enemy_indicies {_ = enemies.remove(index)}
                _ = this.writer.send(response);

                counter += 1;
                counter %= 5;
            }
        });
    }

    pub fn summon(&mut self, position: Vector3<f64>, rotation: Vector2<f32>, typeid: i32) -> (Vec<u8>, Arc<Mutex<Entity>>) {
        let uuid_raw = random::<u128>().to_be_bytes().to_vec();
        let uuid: Arc<str> = Arc::from(UUID(&uuid_raw).decode(0).unwrap().value.as_str());
        let response = (CLIENT_BOUND_PACKETS.summon_entity)(self.latest_entity_id, uuid.clone(), typeid, position, rotation, 0, Vector3 {x:0.0, y:0.0, z:0.0}).unwrap();
        self.latest_entity_id += 1;

        let entity = Arc::new(Mutex::new(Entity {
            eid: self.latest_entity_id - 1,
            id: typeid,
            uuid, position, rotation,
        }));

        self.entity_list.push(entity.clone());
        (response, entity)
    }

    pub fn kill(&mut self, entity: &Arc<Mutex<Entity>>, eid: i32) -> Vec<u8> {
        self.entity_list.retain(|_entity| Arc::ptr_eq(_entity, entity));
        (CLIENT_BOUND_PACKETS.remove_entity)(vec![eid]).unwrap()
    }
}

#[derive(Debug)]
pub struct Bullet {
    pub friendly: bool,
    pub entity: Arc<Mutex<Entity>>,
    pub position: Vector2<f64>,
    pub direction: f32,
}

#[derive(Debug)]
pub struct Enemy {
    pub entity: Arc<Mutex<Entity>>,
    pub position: Vector2<f64>,
    pub target_offset: Vector2<f64>, // offset from player to enemy target to create randomness
    pub health: u16,
    pub tick: u16, // temporary testing value
}

#[derive(Debug)]
pub struct Entity {
    pub id: i32,
    pub eid: i32,
    pub uuid: Arc<str>,
    pub position: Vector3<f64>,
    pub rotation: Vector2<f32>,
}

impl Entity {
    pub fn move_entity(&mut self, position: Vector3<f64>, rotation: Vector2<f32>) -> Vec<u8> {
        let packet = (CLIENT_BOUND_PACKETS.move_entity)(self.eid, Vector3 {
            x: position.x - self.position.x, 
            y: position.y - self.position.y, 
            z: position.z - self.position.z }, rotation, false).unwrap();
        self.position = position;
        self.rotation = rotation;

        packet
    }
}
