use crate::msg::*;
use crossbeam_channel::{bounded, tick, Sender, Receiver, select};

#[derive(Debug, Default)]
pub struct User {
    pub id: String,
    pub hero: String,
    pub isLogin: bool,
    pub isInRoom: bool,
    pub isChooseNGHero: bool,
    pub isStartQueue: bool,
    pub isPreStart: bool,
    pub isPlaying: bool,
}

impl User {
    pub fn success_next(&mut self, tx: &mut Sender<MqttMsg>) {
        if !self.isLogin {
            self.login(tx);
        }
        else if !self.isChooseNGHero {
            self.choose_hero(tx, "freyja".to_owned());
        }
        else if !self.isStartQueue {
            self.start_queue(tx);
        }
    }
    pub fn login(&self, tx: &mut Sender<MqttMsg>) {
        if !self.isLogin {
            let msg = format!(r#"{{"id":"{}"}}"#, self.id);
            let topic = format!("member/{}/send/login", self.id);
            tx.send(MqttMsg{topic:topic, msg:msg});
        }
    }
    pub fn get_login(&mut self) {
        self.isLogin = true;
    }
    pub fn logout(&self, tx: &mut Sender<MqttMsg>) {
        let msg = format!(r#"{{"id":"{}"}}"#, self.id);
        let topic = format!("member/{}/send/logout", self.id);
        tx.send(MqttMsg{topic:topic, msg:msg});
    }
    pub fn get_logout(&mut self, tx: &mut Sender<MqttMsg>) {
        self.isLogin = false;
        self.isChooseNGHero = false;
        self.isStartQueue = false;
        self.isPreStart = false;
        self.isPlaying = false;
    }
    pub fn choose_hero(&self, tx: &mut Sender<MqttMsg>, hero: String) {
        let msg = format!(r#"{{"id":"{}", "hero":"{}"}}"#, self.id, self.hero);
        let topic = format!("member/{}/send/logout", self.id);
        tx.send(MqttMsg{topic:topic, msg:msg});
    }
    pub fn get_choose_hero(&mut self, tx: &mut Sender<MqttMsg>, hero: String) {
        self.hero = hero;
        self.isChooseNGHero = false;
    }
    pub fn choose_random_hero(&mut self, tx: &mut Sender<MqttMsg>) {
        self.isChooseNGHero = true;
    }
    pub fn start_queue(&mut self, tx: &mut Sender<MqttMsg>) {
        self.isStartQueue = false;
    }
    pub fn prestart(&mut self, tx: &mut Sender<MqttMsg>) {
        self.isPreStart = true;
    }
    pub fn afk(&mut self) {
        self.isLogin = false;
        self.isChooseNGHero = false;
        self.isStartQueue = false;
        self.isPreStart = false;
        self.isPlaying = false;
    }
}
