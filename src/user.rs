use log::{info, warn, error, trace};
use crate::msg::*;
use crossbeam_channel::{bounded, tick, Sender, Receiver, select};
use rand::Rng;
use std::cell::RefCell;
use std::rc::Rc;


#[derive(Debug, Default)]
pub struct User {
    pub id: String,
    pub hero: String,
    pub room: String,
    pub isLogin: bool,
    pub isInRoom: bool,
    pub isRoomCreater: bool,
    pub isChooseNGHero: bool,
    pub isStartQueue: bool,
    pub isCanPreStart: bool,
    pub isPreStart: bool,
    pub isPlaying: bool,
}

impl User {
    pub fn next_action(&mut self, tx: &mut Sender<MqttMsg>, room: &String) {
        if !self.isLogin {
            self.login(tx);
        }
        else if !self.isChooseNGHero {
            self.choose_hero(tx, "freyja".to_owned());
        }
        else if !self.isInRoom {
            if room == "" {
                self.create(tx);
            }
            else {
                self.join(tx, room.to_string());
            }
        }
        else if !self.isStartQueue {
            self.start_queue(tx);
        }
        else if self.isCanPreStart {
            self.prestart(tx);
        }
    }
    pub fn back_action(&mut self, tx: &mut Sender<MqttMsg>) {
        if self.isLogin {
            self.logout(tx);
        }
        else if self.isInRoom && self.isRoomCreater {
            self.close(tx);
        }
        else if !self.isStartQueue {
            self.start_queue(tx);
        }
        else if self.isCanPreStart {
            self.prestart(tx);
        }
    }

    pub fn login(&self, tx: &mut Sender<MqttMsg>) {
        if !self.isLogin {
            let msg = format!(r#"{{"id":"{}"}}"#, self.id);
            let topic = format!("member/{}/send/login", self.id);
            tx.send(MqttMsg{topic:topic, msg:msg});
        }
    }

    pub fn join(&self, tx: &mut Sender<MqttMsg>, room: String) {
        if !self.isInRoom {
            let msg = format!(r#"{{"room":"{}", "join":"{}"}}"#, room, self.id);
            let topic = format!("room/{}/send/join", self.id);
            tx.send(MqttMsg{topic:topic, msg:msg});
        }
    }
    pub fn get_join(&mut self, room: String) {
        self.isInRoom = true;
        self.room = room;
    }

    pub fn get_login(&mut self) {
        self.isLogin = true;
    }
    pub fn logout(&self, tx: &mut Sender<MqttMsg>) {
        if self.isLogin {
            let msg = format!(r#"{{"id":"{}"}}"#, self.id);
            let topic = format!("member/{}/send/logout", self.id);
            tx.send(MqttMsg{topic:topic, msg:msg});
        }
    }
    pub fn get_logout(&mut self) {
        self.isLogin = false;
        self.isChooseNGHero = false;
        self.isStartQueue = false;
        self.isPreStart = false;
        self.isPlaying = false;
    }
    pub fn choose_hero(&mut self, tx: &mut Sender<MqttMsg>, hero: String) {
        self.hero = hero;
        if !self.isChooseNGHero {
            let msg = format!(r#"{{"id":"{}", "hero":"{}"}}"#, self.id, self.hero);
            let topic = format!("member/{}/send/choose_hero", self.id);
            tx.send(MqttMsg{topic:topic, msg:msg});
        }
    }
    pub fn get_choose_hero(&mut self, hero: String) {
        self.hero = hero;
        self.isChooseNGHero = true;
    }
    pub fn choose_random_hero(&mut self, tx: &mut Sender<MqttMsg>) {
    }
    pub fn create(&mut self, tx: &mut Sender<MqttMsg>) {
        if !self.isInRoom {
            let msg = format!(r#"{{"id":"{}"}}"#, self.id);
            let topic = format!("room/{}/send/create", self.id);
            tx.send(MqttMsg{topic:topic, msg:msg});
            self.room = self.id.clone();
        }
    }
    pub fn get_create(&mut self) {
        self.isInRoom = true;
        self.room = self.id.clone();
        self.isRoomCreater = true;
    }
    pub fn close(&self, tx: &mut Sender<MqttMsg>) {
        if self.isInRoom {
            let msg = format!(r#"{{"id":"{}"}}"#, self.id);
            let topic = format!("room/{}/send/close", self.id);
            tx.send(MqttMsg{topic:topic, msg:msg});
        }
    }
    pub fn get_close(&mut self) {
        self.isInRoom = false;
        self.room = "".to_owned();
    }
    pub fn start_queue(&mut self, tx: &mut Sender<MqttMsg>) {
        if self.isInRoom && !self.isRoomCreater {
            self.isStartQueue = true;
        }
        if !self.isStartQueue {
            let msg = format!(r#"{{"id":"{}", "action":"start queue"}}"#, self.id);
            let topic = format!("room/{}/send/start_queue", self.room);
            tx.send(MqttMsg{topic:topic, msg:msg});
        }
    }
    pub fn get_start_queue(&mut self) {
        self.isStartQueue = true;
    }
    pub fn prestart(&mut self, tx: &mut Sender<MqttMsg>) {
        if self.isCanPreStart && !self.isPreStart {
            self.isPreStart = true;
            let mut rng = rand::thread_rng();
            let msg = format!(r#"{{"room":"{}", "id":"{}", "accept":true}}"#, self.room, self.id);
            let topic = format!("room/{}/send/prestart", self.id);
            tx.try_send(MqttMsg{topic:topic, msg:msg}).unwrap();
        }
    }
    pub fn get_prestart(&mut self, res: bool) {
        self.isCanPreStart = res;
        if res == false {
            self.isPreStart = false;
        }
    }
    pub fn invite(&mut self, tx: &mut Sender<MqttMsg>) {
        
    }
    pub fn get_invite(&mut self) {
        
    }
    pub fn afk(&mut self) {
        self.isLogin = false;
        self.isChooseNGHero = false;
        self.isStartQueue = false;
        self.isPreStart = false;
        self.isPlaying = false;
    }
}
