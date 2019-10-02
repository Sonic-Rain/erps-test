use serde_json::{self, Result, Value};
use std::env;
use std::io::{self, Write};
use serde_derive::{Serialize, Deserialize};
use std::io::{ErrorKind};
use log::{info, warn, error, trace};
use std::thread;
use std::time::{Duration, Instant};

use ::futures::Future;
use mysql;
use std::sync::{Arc, Mutex, Condvar, RwLock};
use crossbeam_channel::{bounded, tick, Sender, Receiver, select};
use std::collections::{HashMap, BTreeMap};
use std::cell::RefCell;
use std::rc::Rc;
use failure::Error;
use std::process::Command;
use indexmap::IndexMap;

use crate::user::*;
use crate::msg::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoginRes {
    pub msg: String,
}
#[derive(Clone, Debug)]
pub struct LoginMsg {
    pub id: String,
    pub msg: String,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogoutRes {
    pub msg: String,
}
#[derive(Clone, Debug)]
pub struct LogoutMsg {
    pub id: String,
    pub msg: String,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateRoomRes {
    pub msg: String,
}
#[derive(Clone, Debug)]
pub struct CreateRoomMsg {
    pub id: String,
    pub msg: String,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CloseRoomRes {
    pub msg: String,
}
#[derive(Clone, Debug)]
pub struct CloseRoomMsg {
    pub room: String,
    pub msg: String,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserNGHeroRes {
    pub id: String,
    pub hero: String,
}
#[derive(Clone, Debug)]
pub struct UserNGHeroMsg {
    pub id: String,
    pub hero: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InviteRes {
    pub msg: String,
}
#[derive(Clone, Debug)]
pub struct InviteMsg {
    pub id: String,
    pub msg: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StartQueueRes {
    pub msg: String,
}
#[derive(Clone, Debug)]
pub struct StartQueueMsg {
    pub id: String,
    pub msg: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PreStartRes {
    pub msg: String,
}
#[derive(Clone, Debug)]
pub struct PreStartMsg {
    pub id: String,
    pub msg: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JoinRes {
    pub room: String,
    pub msg: String,
}
#[derive(Clone, Debug)]
pub struct JoinMsg {
    pub id: String,
    pub room: String,
    pub msg: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StartRes {
    pub game: u32,
    pub room: String,
    pub msg: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StartGameRes {
    pub game: u32,
    pub member: Vec<HeroCell>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct HeroCell {
    pub id: String,
    pub team: u16,
    pub name: String,
    pub hero: String,
    pub buff: BTreeMap<String, f32>,
    pub tags: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameSingalRes {
    pub game: u32,
}

pub enum UserEvent {
    Login(LoginMsg),
    Logout(LogoutMsg),
    Create(CreateRoomMsg),
    Close(CloseRoomMsg),
    ChooseNGHero(UserNGHeroMsg),
    Invite(InviteMsg),
    StartQueue(StartQueueMsg),
    PreStart(PreStartMsg),
    Join(JoinMsg),
    StartGame(StartGameRes),
    Start(StartRes),
    GameSingal(GameSingalRes),
}

fn get_user(id: &String, users: &BTreeMap<String, Rc<RefCell<User>>>) -> Option<Rc<RefCell<User>>> {
    let u = users.get(id);
    if let Some(u) = u {
        return Some(Rc::clone(u))
    }
    None
}

pub fn init(msgtx: Sender<MqttMsg>) -> Sender<UserEvent> {
    let (tx, rx):(Sender<UserEvent>, Receiver<UserEvent>) = bounded(1000);
    let start = Instant::now();
    let update500ms = tick(Duration::from_millis(500));
    let update100ms = tick(Duration::from_millis(100));
    
    
    thread::spawn(move || {
        let mut rooms: IndexMap<String, Rc<RefCell<RoomRecord>>> = IndexMap::new();
        let mut TotalUsers: BTreeMap<String, Rc<RefCell<User>>> = BTreeMap::new();
        for i in 0..4 {
            TotalUsers.insert(i.to_string(),
                Rc::new(RefCell::new(
                User {
                    id: i.to_string(),
                    hero: "".to_string(),
                    ..Default::default()
                }
            )));
        }
        let mut tx = msgtx.clone();
        loop {
            select! {
                recv(update500ms) -> _ => {
                    for (i, u) in &mut TotalUsers {
                        u.borrow_mut().next_action(&mut tx, &mut rooms);
                    }
                }
                recv(rx) -> d => {
                    if let Ok(d) = d {
                        match d {
                            UserEvent::Start(x) => {
                                
                            },
                            UserEvent::GameSingal(x) => {
                                println!("StartGame {:?}", TotalUsers);
                                let mut tx = tx.clone();
                                thread::spawn(move || {
                                    thread::sleep_ms(3000);
                                    tx.try_send(MqttMsg{topic:format!("game/{}/send/start_game", x.game), 
                                                msg: format!(r#"{{"game":{},"action":"init"}}"#, x.game)}).unwrap();
                                });
                            },
                            UserEvent::StartGame(x) => {
                                println!("StartGame {:?}", TotalUsers);
                            },
                            UserEvent::Join(x) => {
                                if x.msg == "ok" {
                                    let u = get_user(&x.id, &TotalUsers);
                                    if let Some(u) = u {
                                        u.borrow_mut().get_join(x.room.clone());
                                    }
                                    let r = rooms.get(&x.room);
                                    if let Some(r) = r {
                                        r.borrow_mut().ids.push(x.id.clone());
                                    }
                                }
                            },
                            UserEvent::Login(x) => {
                                let u = get_user(&x.id, &TotalUsers);
                                if let Some(u) = u {
                                    u.borrow_mut().get_login();
                                }
                            },
                            UserEvent::Logout(x) => {
                                let u = get_user(&x.id, &TotalUsers);
                                if let Some(u) = u {
                                    u.borrow_mut().get_logout();
                                }
                            },
                            UserEvent::Create(x) => {
                                let u = get_user(&x.id, &TotalUsers);
                                if let Some(u) = u {
                                    u.borrow_mut().get_create();
                                }
                            },
                            UserEvent::Close(x) => {
                                let u = get_user(&x.room, &TotalUsers);
                                if let Some(u) = u {
                                    u.borrow_mut().get_close();
                                }
                            },
                            UserEvent::ChooseNGHero(x) => {
                                let u = get_user(&x.id, &TotalUsers);
                                if let Some(u) = u {
                                    u.borrow_mut().get_choose_hero(x.hero);
                                }
                            },
                            UserEvent::Invite(x) => {
                                let u = get_user(&x.id, &TotalUsers);
                                if let Some(u) = u {
                                    u.borrow_mut().get_invite();
                                }
                            },
                            UserEvent::StartQueue(x) => {
                                let u = get_user(&x.id, &TotalUsers);
                                if let Some(u) = u {
                                    u.borrow_mut().get_start_queue();
                                }
                                println!("StartQueue {:#?}", TotalUsers);
                            },
                            UserEvent::PreStart(x) => {
                                if x.msg == "stop queue" {
                                    let u = get_user(&x.id, &TotalUsers);
                                    if let Some(u) = u {
                                        u.borrow_mut().get_prestart(false);
                                    }
                                }
                                else {
                                    let r = rooms.get(&x.id);
                                    if let Some(r) = r {
                                        for id in &r.borrow().ids {
                                            let u = get_user(&id, &TotalUsers);
                                            if let Some(u) = u {
                                                u.borrow_mut().get_prestart(true);
                                            }
                                        }
                                    }
                                    println!("PreStart");
                                    /*let u = get_user(&x.id, &TotalUsers);
                                    if let Some(u) = u {
                                        u.borrow_mut().get_prestart(true);
                                    }*/
                                }
                            },
                        }
                    }
                }
            }
        }
    });
    tx
}

pub fn login(id: String, v: Value, sender: Sender<UserEvent>)
 -> std::result::Result<(), Error>
{
    let data: LoginRes = serde_json::from_value(v)?;
    sender.send(UserEvent::Login(LoginMsg{id:id, msg:data.msg}));
    Ok(())
}

pub fn logout(id: String, v: Value, sender: Sender<UserEvent>)
 -> std::result::Result<(), Error>
{
    let data: LogoutRes = serde_json::from_value(v)?;
    sender.send(UserEvent::Logout(LogoutMsg{id:id, msg:data.msg}));
    Ok(())
}

pub fn create(id: String, v: Value, sender: Sender<UserEvent>)
 -> std::result::Result<(), Error>
{
    let data: CreateRoomRes = serde_json::from_value(v)?;
    sender.send(UserEvent::Create(CreateRoomMsg{id:id, msg:data.msg}));
    Ok(())
}

pub fn close(id: String, v: Value, sender: Sender<UserEvent>)
 -> std::result::Result<(), Error>
{
    let data: LogoutRes = serde_json::from_value(v)?;
    sender.send(UserEvent::Logout(LogoutMsg{id:id, msg:data.msg}));
    Ok(())
}

pub fn choose_hero(id: String, v: Value, sender: Sender<UserEvent>)
 -> std::result::Result<(), Error>
{
    let data: UserNGHeroRes = serde_json::from_value(v)?;
    sender.send(UserEvent::ChooseNGHero(UserNGHeroMsg{id:id, hero: data.hero}));
    Ok(())
}

pub fn start_queue(id: String, v: Value, sender: Sender<UserEvent>)
 -> std::result::Result<(), Error>
{
    let data: StartQueueRes = serde_json::from_value(v)?;
    sender.send(UserEvent::StartQueue(StartQueueMsg{id:id, msg:data.msg}));
    Ok(())
}

pub fn prestart(id: String, v: Value, sender: Sender<UserEvent>)
 -> std::result::Result<(), Error>
{
    let data: StartQueueRes = serde_json::from_value(v)?;
    sender.send(UserEvent::PreStart(PreStartMsg{id:id, msg:data.msg}));
    Ok(())
}

pub fn join(id: String, v: Value, sender: Sender<UserEvent>)
 -> std::result::Result<(), Error>
{
    let data: JoinRes = serde_json::from_value(v)?;
    sender.send(UserEvent::Join(JoinMsg{id:id, room: data.room, msg:data.msg}));
    Ok(())
}

pub fn start(id: String, v: Value, sender: Sender<UserEvent>)
 -> std::result::Result<(), Error>
{
    let data: StartRes = serde_json::from_value(v)?;
    sender.send(UserEvent::Start(data));
    Ok(())
}

pub fn start_game(id: String, v: Value, sender: Sender<UserEvent>)
 -> std::result::Result<(), Error>
{
    let data: StartGameRes = serde_json::from_value(v)?;
    sender.send(UserEvent::StartGame(data));
    Ok(())
}

pub fn game_singal(id: String, v: Value, sender: Sender<UserEvent>)
 -> std::result::Result<(), Error>
{
    let data: GameSingalRes = serde_json::from_value(v)?;
    sender.send(UserEvent::GameSingal(data));
    Ok(())
}

