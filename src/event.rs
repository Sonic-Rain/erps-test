use mqtt;
use mqtt::packet::*;
use serde_json::{self, Result, Value};
use mqtt::{Decodable, Encodable, QualityOfService};
use mqtt::{TopicFilter, TopicName};
use std::env;
use std::io::{self, Write};
use serde_derive::{Serialize, Deserialize};
use std::io::{Error, ErrorKind};
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

use std::process::Command;

use crate::user::User;
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
    pub id: String, 
    pub room: String,
    pub msg: String,
}
#[derive(Clone, Debug)]
pub struct CreateRoomMsg {
    pub id: String, 
    pub room: String,
    pub msg: String,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CloseRoomRes {
    pub room: String,
    pub msg: String,
}
#[derive(Clone, Debug)]
pub struct CloseRoomMsg {
    pub id: String, 
    pub room: String,
    pub msg: String,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserNGHeroRes {
    pub hero: String,
    pub msg: String,
}
#[derive(Clone, Debug)]
pub struct UserNGHeroMsg {
    pub id: String,
    pub hero: String,
    pub msg: String,
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

pub enum UserEvent {
    Login(LoginMsg),
    Logout(LogoutMsg),
    Create(CreateRoomMsg),
    Close(CloseRoomMsg),
    ChooseNGHero(UserNGHeroMsg),
    Invite(InviteMsg),
    StartQueue(StartQueueMsg),
}

pub fn init(msgtx: Sender<MqttMsg>) -> Sender<UserEvent> {
    let (tx, rx):(Sender<UserEvent>, Receiver<UserEvent>) = bounded(1000);
    let start = Instant::now();
    let update200ms = tick(Duration::from_millis(200));
    let update100ms = tick(Duration::from_millis(100));
    
    thread::spawn(move || {
        let mut TotalUsers: Vec<Rc<RefCell<User>>> = vec![];
        for i in 0..10 {
            TotalUsers.push(Rc::new(RefCell::new(
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
                recv(update200ms) -> _ => {
                    for u in &mut TotalUsers {
                        u.borrow_mut().login(&mut tx);
                    }
                }
                recv(rx) -> d => {
                    if let Ok(d) = d {
                        match d {
                            UserEvent::Login(x) => {
                                for u in &mut TotalUsers {
                                    if u.borrow().id == x.id {
                                        u.borrow_mut().get_login();
                                        break;
                                    }
                                }
                            },
                            UserEvent::Logout(x) => {
                                for u in &mut TotalUsers {
                                    if u.borrow().id == x.id {
                                        u.borrow_mut().isLogin = false;
                                        break;
                                    }
                                }
                            },
                            UserEvent::Create(x) => {
                                for u in &mut TotalUsers {
                                    if u.borrow().id == x.id {
                                        u.borrow_mut().isInRoom = false;
                                        break;
                                    }
                                }
                            },
                            UserEvent::Close(x) => {
                                for u in &mut TotalUsers {
                                    if u.borrow().id == x.id {
                                        u.borrow_mut().isInRoom = false;
                                        break;
                                    }
                                }
                            },
                            UserEvent::ChooseNGHero(x) => {
                                for u in &mut TotalUsers {
                                    if u.borrow().id == x.id {
                                        u.borrow_mut().hero = x.hero;
                                        break;
                                    }
                                }
                            },
                            UserEvent::Invite(x) => {
                                for u in &mut TotalUsers {
                                    if u.borrow().id == x.id {
                                        break;
                                    }
                                }
                            },
                            UserEvent::StartQueue(x) => {
                                for u in &mut TotalUsers {
                                    if u.borrow().id == x.id {
                                        u.borrow_mut().isStartQueue = true;
                                        break;
                                    }
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
 -> std::result::Result<(), std::io::Error>
{
    let data: LoginRes = serde_json::from_value(v)?;
    sender.send(UserEvent::Login(LoginMsg{id:id, msg:data.msg}));
    Ok(())
}

pub fn logout(id: String, v: Value, sender: Sender<UserEvent>)
 -> std::result::Result<(), std::io::Error>
{
    let data: LogoutRes = serde_json::from_value(v)?;
    sender.send(UserEvent::Logout(LogoutMsg{id:id, msg:data.msg}));
    Ok(())
}

pub fn create(id: String, v: Value, sender: Sender<UserEvent>)
 -> std::result::Result<(), std::io::Error>
{
    let data: LogoutRes = serde_json::from_value(v)?;
    sender.send(UserEvent::Logout(LogoutMsg{id:id, msg:data.msg}));
    Ok(())
}

pub fn close(id: String, v: Value, sender: Sender<UserEvent>)
 -> std::result::Result<(), std::io::Error>
{
    let data: LogoutRes = serde_json::from_value(v)?;
    sender.send(UserEvent::Logout(LogoutMsg{id:id, msg:data.msg}));
    Ok(())
}

pub fn choose_hero(id: String, v: Value, sender: Sender<UserEvent>)
 -> std::result::Result<(), std::io::Error>
{
    let data: UserNGHeroRes = serde_json::from_value(v)?;
    sender.send(UserEvent::ChooseNGHero(UserNGHeroMsg{id:id, hero: data.hero, msg:data.msg}));
    Ok(())
}

pub fn start_queue(id: String, v: Value, sender: Sender<UserEvent>)
 -> std::result::Result<(), std::io::Error>
{
    let data: StartQueueRes = serde_json::from_value(v)?;
    sender.send(UserEvent::StartQueue(StartQueueMsg{id:id, msg:data.msg}));
    Ok(())
}
