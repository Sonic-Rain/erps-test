use serde_json::{self, Result, Value};
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

pub enum UserEvent {
    Login(LoginMsg),
    Logout(LogoutMsg),
    Create(CreateRoomMsg),
    Close(CloseRoomMsg),
    ChooseNGHero(UserNGHeroMsg),
    Invite(InviteMsg),
    StartQueue(StartQueueMsg),
    PreStart(PreStartMsg),
}

pub fn init(msgtx: Sender<MqttMsg>) -> Sender<UserEvent> {
    let (tx, rx):(Sender<UserEvent>, Receiver<UserEvent>) = bounded(1000);
    let start = Instant::now();
    let update500ms = tick(Duration::from_millis(500));
    let update100ms = tick(Duration::from_millis(100));
    
    thread::spawn(move || {
        let mut TotalUsers: Vec<Rc<RefCell<User>>> = vec![];
        for i in 0..4 {
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
                recv(update500ms) -> _ => {
                    for u in &mut TotalUsers {
                        u.borrow_mut().next_action(&mut tx);
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
                                        u.borrow_mut().get_logout();
                                        break;
                                    }
                                }
                            },
                            UserEvent::Create(x) => {
                                for u in &mut TotalUsers {
                                    if u.borrow().id == x.id {
                                        u.borrow_mut().get_create();
                                        break;
                                    }
                                }
                            },
                            UserEvent::Close(x) => {
                                for u in &mut TotalUsers {
                                    if u.borrow().room == x.room {
                                        u.borrow_mut().get_close();
                                    }
                                }
                            },
                            UserEvent::ChooseNGHero(x) => {
                                for u in &mut TotalUsers {
                                    if u.borrow().id == x.id {
                                        u.borrow_mut().get_choose_hero(x.hero);
                                        break;
                                    }
                                }
                            },
                            UserEvent::Invite(x) => {
                                for u in &mut TotalUsers {
                                    if u.borrow().id == x.id {
                                        u.borrow_mut().get_invite();
                                        break;
                                    }
                                }
                            },
                            UserEvent::StartQueue(x) => {
                                for u in &mut TotalUsers {
                                    if u.borrow().id == x.id {
                                        u.borrow_mut().get_start_queue();
                                        break;
                                    }
                                }
                            },
                            UserEvent::PreStart(x) => {
                                if x.msg == "stop queue" {
                                    for u in &mut TotalUsers {
                                        if u.borrow().id == x.id {
                                            u.borrow_mut().get_prestart(false);
                                            break;
                                        }
                                    }
                                }
                                else {
                                    for u in &mut TotalUsers {
                                        if u.borrow().id == x.id {
                                            u.borrow_mut().get_prestart(true);
                                            break;
                                        }
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
    let data: CreateRoomRes = serde_json::from_value(v)?;
    sender.send(UserEvent::Create(CreateRoomMsg{id:id, msg:data.msg}));
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
    sender.send(UserEvent::ChooseNGHero(UserNGHeroMsg{id:id, hero: data.hero}));
    Ok(())
}

pub fn start_queue(id: String, v: Value, sender: Sender<UserEvent>)
 -> std::result::Result<(), std::io::Error>
{
    let data: StartQueueRes = serde_json::from_value(v)?;
    sender.send(UserEvent::StartQueue(StartQueueMsg{id:id, msg:data.msg}));
    Ok(())
}

pub fn prestart(id: String, v: Value, sender: Sender<UserEvent>)
 -> std::result::Result<(), std::io::Error>
{
    let data: StartQueueRes = serde_json::from_value(v)?;
    sender.send(UserEvent::PreStart(PreStartMsg{id:id, msg:data.msg}));
    Ok(())
}
