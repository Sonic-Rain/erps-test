  #![allow(warnings)]
use log::{info, warn, error, trace};

use std::env;
use std::io::Write;
use failure::Error;
use std::net::TcpStream;
use std::str;
use clap::{App, Arg};
use uuid::Uuid;
use rumqtt::{MqttClient, MqttOptions, QoS};

use std::thread;
use std::time::Duration;
use log::Level;
use serde_json::{self, Value};
use regex::Regex;
use ::futures::Future;
use mysql;

use crossbeam_channel::{bounded, tick, Sender, Receiver, select};

mod msg;
mod event;
mod user;
use crate::msg::*;
use crate::event::*;


fn generate_client_id() -> String {
    format!("ERPS_TEST_{}", Uuid::new_v4())
}

fn main() -> std::result::Result<(), Error> {
    // configure logging
    env::set_var("RUST_LOG", env::var_os("RUST_LOG").unwrap_or_else(|| "info".into()));
    env_logger::init();

    let matches = App::new("erps")
        .author("damody <t1238142000@gmail.com>")
        .arg(
            Arg::with_name("SERVER")
                .short("S")
                .long("server")
                .takes_value(true)
                .help("MQTT server address (127.0.0.1)"),
        ).arg(
            Arg::with_name("PORT")
                .short("P")
                .long("port")
                .takes_value(true)
                .help("MQTT server port (1883)"),
        ).arg(
            Arg::with_name("USER_NAME")
                .short("u")
                .long("username")
                .takes_value(true)
                .help("Login user name"),
        ).arg(
            Arg::with_name("PASSWORD")
                .short("p")
                .long("password")
                .takes_value(true)
                .help("Password"),
        ).arg(
            Arg::with_name("CLIENT_ID")
                .short("i")
                .long("client-identifier")
                .takes_value(true)
                .help("Client identifier"),
        ).get_matches();

    let server_addr = matches.value_of("SERVER").unwrap_or("127.0.0.1").to_owned();
    let server_port = matches.value_of("PORT").unwrap_or("1883").to_owned();
    let client_id = matches
        .value_of("CLIENT_ID")
        .map(|x| x.to_owned())
        .unwrap_or_else(generate_client_id);
    let mut mqtt_options = MqttOptions::new(client_id.as_str(), server_addr.as_str(), server_port.parse::<u16>().unwrap());
    mqtt_options = mqtt_options.set_keep_alive(100);
    mqtt_options = mqtt_options.set_request_channel_capacity(10000);
    mqtt_options = mqtt_options.set_notification_channel_capacity(10000);
    let (mut mqtt_client, notifications) = MqttClient::start(mqtt_options.clone()).unwrap();
    mqtt_client.subscribe("member/+/res/login", QoS::AtLeastOnce).unwrap();
    mqtt_client.subscribe("member/+/res/logout", QoS::AtLeastOnce).unwrap();
    mqtt_client.subscribe("member/+/res/choose_hero", QoS::AtLeastOnce).unwrap();

    mqtt_client.subscribe("room/+/res/create", QoS::AtLeastOnce).unwrap();
    mqtt_client.subscribe("room/+/res/close", QoS::AtLeastOnce).unwrap();
    mqtt_client.subscribe("room/+/res/start_queue", QoS::AtLeastOnce).unwrap();
    mqtt_client.subscribe("room/+/res/cancel_queue", QoS::AtLeastOnce).unwrap();
    mqtt_client.subscribe("room/+/res/invite", QoS::AtLeastOnce).unwrap();
    mqtt_client.subscribe("room/+/res/join", QoS::AtLeastOnce).unwrap();
    mqtt_client.subscribe("room/+/res/accept_join", QoS::AtLeastOnce).unwrap();
    mqtt_client.subscribe("room/+/res/kick", QoS::AtLeastOnce).unwrap();
    mqtt_client.subscribe("room/+/res/leave", QoS::AtLeastOnce).unwrap();
    mqtt_client.subscribe("room/+/res/prestart", QoS::AtLeastOnce).unwrap();
    mqtt_client.subscribe("room/+/res/start", QoS::AtLeastOnce).unwrap();

    mqtt_client.subscribe("game/+/res/game_singal", QoS::AtLeastOnce).unwrap();
    mqtt_client.subscribe("game/+/res/game_over", QoS::AtLeastOnce).unwrap();
    mqtt_client.subscribe("game/+/res/start_game", QoS::AtLeastOnce).unwrap();
    mqtt_client.subscribe("game/+/res/choose", QoS::AtLeastOnce).unwrap();
    mqtt_client.subscribe("game/+/res/exit", QoS::AtLeastOnce).unwrap();

    let (tx, rx):(Sender<MqttMsg>, Receiver<MqttMsg>) = bounded(10000);
    let mut sender: Sender<UserEvent> = event::init(tx.clone());
    thread::sleep_ms(100);
    thread::spawn(move || {
        let mut pkid = 100;
        let mut mqtt_options = MqttOptions::new(generate_client_id(), server_addr, server_port.parse::<u16>().unwrap());
        mqtt_options = mqtt_options.set_keep_alive(100);
        mqtt_options = mqtt_options.set_request_channel_capacity(10000);
        mqtt_options = mqtt_options.set_notification_channel_capacity(10000);
        let (mut mqtt_client, notifications) = MqttClient::start(mqtt_options.clone()).unwrap();
        loop {
            select! {
                recv(rx) -> d => {
                    let handle = || -> Result<(), Error> {
                        if let Ok(d) = d {
                            match mqtt_client.publish(d.topic, QoS::AtLeastOnce, false, d.msg) {
                                Ok(_) => {},
                                Err(x) => {
                                    println!("publish failed!!!!");
                                }
                            }
                        }
                        Ok(())
                    };
                    if let Err(msg) = handle() {
                        println!("{:?}", msg);
                    }
                }
            }
        }
    });
    
    
    let relogin = Regex::new(r"\w+/(\w+)/res/login").unwrap();
    let relogout = Regex::new(r"\w+/(\w+)/res/logout").unwrap();
    let recreate = Regex::new(r"\w+/(\w+)/res/create").unwrap();
    let reclose = Regex::new(r"\w+/(\w+)/res/close").unwrap();
    let restart_queue = Regex::new(r"\w+/(\w+)/res/start_queue").unwrap();
    let recancel_queue = Regex::new(r"\w+/(\w+)/res/cancel_queue").unwrap();
    let represtart = Regex::new(r"\w+/(\w+)/res/prestart").unwrap();
    let reinvite = Regex::new(r"\w+/(\w+)/res/invite").unwrap();
    let rejoin = Regex::new(r"\w+/(\w+)/res/join").unwrap();
    let rechoosehero = Regex::new(r"\w+/(\w+)/res/choose_hero").unwrap();
    let restart_game = Regex::new(r"\w+/(\w+)/res/start_game").unwrap();
    let restart = Regex::new(r"\w+/(\w+)/res/start").unwrap();
    let regame_singal = Regex::new(r"\w+/(\w+)/res/game_singal").unwrap();

    loop {
        use rumqtt::Notification::Publish;
        select! {
            recv(notifications) -> notification => {
                let handle = || -> Result<(), Error> {
                    if let Ok(x) = notification {
                        if let Publish(x) = x {
                            //println!("{:?}", x);
                            let payload = x.payload;
                            let msg = match str::from_utf8(&payload[..]) {
                                Ok(msg) => msg,
                                Err(err) => {
                                    return Err(failure::err_msg(format!("Failed to decode publish message {:?}", err)));
                                }
                            };
                            let topic_name = x.topic_name.as_str();
                            let vo : serde_json::Result<Value> = serde_json::from_str(msg);
                            if let Ok(v) = vo {
                                if relogin.is_match(topic_name) {
                                    let cap = relogin.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    info!("login: userid: {} json: {:?}", userid, v);
                                    event::login(userid, v, sender.clone())?;
                                }
                                else if relogout.is_match(topic_name) {
                                    let cap = relogout.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    info!("logout: userid: {} json: {:?}", userid, v);
                                    event::logout(userid, v, sender.clone())?;
                                }
                                else if recreate.is_match(topic_name) {
                                    let cap = recreate.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    info!("create: userid: {} json: {:?}", userid, v);
                                    event::create(userid, v, sender.clone())?;
                                }
                                else if reclose.is_match(topic_name) {
                                    let cap = reclose.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    info!("close: userid: {} json: {:?}", userid, v);
                                    event::close(userid, v, sender.clone())?;
                                }
                                else if rejoin.is_match(topic_name) {
                                    let cap = rejoin.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    info!("join: userid: {} json: {:?}", userid, v);
                                    event::join(userid, v, sender.clone())?;
                                }
                                else if restart_queue.is_match(topic_name) {
                                    let cap = restart_queue.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    info!("start_queue: userid: {} json: {:?}", userid, v);
                                    event::start_queue(userid, v, sender.clone())?;
                                }
                                else if rechoosehero.is_match(topic_name) {
                                    let cap = rechoosehero.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    info!("choose hero: userid: {} json: {:?}", userid, v);
                                    event::choose_hero(userid, v, sender.clone())?;
                                }
                                else if represtart.is_match(topic_name) {
                                    let cap = represtart.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    info!("prestart hero: userid: {} json: {:?}", userid, v);
                                    event::prestart(userid, v, sender.clone())?;
                                }
                                else if restart_game.is_match(topic_name) {
                                    let cap = restart_game.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    info!("start_game hero: userid: {} json: {:?}", userid, v);
                                    event::start_game(userid, v, sender.clone())?;
                                }
                                else if restart.is_match(topic_name) {
                                    let cap = restart.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    info!("start hero: userid: {} json: {:?}", userid, v);
                                    event::start(userid, v, sender.clone())?;
                                }
                                else if regame_singal.is_match(topic_name) {
                                    let cap = regame_singal.captures(topic_name).unwrap();
                                    let userid = cap[1].to_string();
                                    info!("game_singal: userid: {} json: {:?}", userid, v);
                                    event::game_singal(userid, v, sender.clone())?;
                                }
                                else {
                                    warn!("Topic Error {}", topic_name);
                                }
                            } else {
                                warn!("Json Parser error");
                            };
                        }
                    }
                    Ok(())
                };
                if let Err(msg) = handle() {
                    println!("{:?}", msg);
                }
            }
        }
    }
    Ok(())
}
