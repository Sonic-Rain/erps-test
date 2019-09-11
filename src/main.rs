  #![allow(warnings)]
use log::{info, warn, error, trace};

use std::env;
use std::io::Write;
use std::io::Error;
use std::net::TcpStream;
use std::str;
use clap::{App, Arg};
use uuid::Uuid;

use mqtt::control::variable_header::ConnectReturnCode;
use mqtt::packet::*;
use mqtt::{Decodable, Encodable, QualityOfService};
use mqtt::{TopicFilter, TopicName};

use std::thread;
use std::time::Duration;
use log::Level;
use serde_json::{self, Result, Value};
use regex::Regex;

use ::futures::Future;
use mysql;

use crossbeam_channel::{bounded, tick, Sender, Receiver, select};

mod msg;
mod event;
mod user;
use crate::msg::*;

fn generate_client_id() -> String {
    format!("ERPS_TEST_{}", Uuid::new_v4())
}

fn main() -> std::result::Result<(), std::io::Error> {
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
                .help("MQTT server address (host:port)"),
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

    let server_addr = matches.value_of("SERVER").unwrap_or("127.0.0.1:1883");
    let client_id = matches
        .value_of("CLIENT_ID")
        .map(|x| x.to_owned())
        .unwrap_or_else(generate_client_id);
    let mut channel_filters: Vec<(TopicFilter, QualityOfService)> = vec![
        (TopicFilter::new("member/+/res/login").unwrap(), QualityOfService::Level2),
        (TopicFilter::new("member/+/res/logout").unwrap(), QualityOfService::Level1),

        (TopicFilter::new("room/+/res/create").unwrap(), QualityOfService::Level1),
        (TopicFilter::new("room/+/res/close").unwrap(), QualityOfService::Level1),
        (TopicFilter::new("room/+/res/start_queue").unwrap(), QualityOfService::Level1),
        (TopicFilter::new("room/+/res/cancel_queue").unwrap(), QualityOfService::Level1),        
        (TopicFilter::new("room/+/res/invite").unwrap(), QualityOfService::Level1),
        (TopicFilter::new("room/+/res/join").unwrap(), QualityOfService::Level1),
        (TopicFilter::new("room/+/res/accept_join").unwrap(), QualityOfService::Level1),
        (TopicFilter::new("room/+/res/kick").unwrap(), QualityOfService::Level1),
        (TopicFilter::new("room/+/res/leave").unwrap(), QualityOfService::Level1),
        (TopicFilter::new("room/+/res/prestart").unwrap(), QualityOfService::Level1),
        (TopicFilter::new("room/+/res/start").unwrap(), QualityOfService::Level1),

        (TopicFilter::new("game/+/res/game_over").unwrap(), QualityOfService::Level1),
        (TopicFilter::new("game/+/res/start_game").unwrap(), QualityOfService::Level1),
        (TopicFilter::new("game/+/res/choose").unwrap(), QualityOfService::Level1),
        (TopicFilter::new("game/+/res/exit").unwrap(), QualityOfService::Level1),
        (TopicFilter::new("game/+/res/choose").unwrap(), QualityOfService::Level1),
    ];

    let keep_alive = 100;

    info!("Connecting to {:?} ... ", server_addr);
    let mut stream = TcpStream::connect(server_addr).unwrap();
    info!("Connected!");

    info!("Client identifier {:?}", client_id);
    let mut conn = ConnectPacket::new("MQTT", client_id);
    conn.set_clean_session(true);
    conn.set_keep_alive(keep_alive);
    let mut buf = Vec::new();
    conn.encode(&mut buf).unwrap();
    stream.write_all(&buf[..]).unwrap();

    let connack = ConnackPacket::decode(&mut stream).unwrap();
    trace!("CONNACK {:?}", connack);

    if connack.connect_return_code() != ConnectReturnCode::ConnectionAccepted {
        panic!(
            "Failed to connect to server, return code {:?}",
            connack.connect_return_code()
        );
    }

    // const CHANNEL_FILTER: &'static str = "typing-speed-test.aoeu.eu";
    trace!("Applying channel filters {:?} ...", channel_filters);
    let sub = SubscribePacket::new(10, channel_filters);
    let mut buf = Vec::new();
    sub.encode(&mut buf).unwrap();
    stream.write_all(&buf[..]).unwrap();

    loop {
        let packet = match VariablePacket::decode(&mut stream) {
            Ok(pk) => pk,
            Err(err) => {
                error!("Error in receiving packet {:?}", err);
                continue;
            }
        };
        trace!("PACKET {:?}", packet);

        if let VariablePacket::SubackPacket(ref ack) = packet {
            if ack.packet_identifier() != 10 {
                panic!("SUBACK packet identifier not match");
            }
            info!("Subscribed!");
            break;
        }
    }

    let mut stream_clone = stream.try_clone().unwrap();
    thread::spawn(move || {
        let mut last_ping_time = 0;
        let mut next_ping_time = last_ping_time + (keep_alive as f32 * 0.9) as i64;
        loop {
            let current_timestamp = time::get_time().sec;
            if keep_alive > 0 && current_timestamp >= next_ping_time {
                let pingreq_packet = PingreqPacket::new();

                let mut buf = Vec::new();
                pingreq_packet.encode(&mut buf).unwrap();
                stream_clone.write_all(&buf[..]).unwrap();

                last_ping_time = current_timestamp;
                next_ping_time = last_ping_time + (keep_alive as f32 * 0.9) as i64;
                thread::sleep(Duration::new((keep_alive / 2) as u64, 0));
            }
        }
    });

    let (tx, rx):(Sender<MqttMsg>, Receiver<MqttMsg>) = bounded(1000);
    let mut stream2 = stream.try_clone().unwrap();
    thread::spawn(move || {
        let mut pkid = 100;
        loop {            
            select! {
                recv(rx) -> d => {
                    if let Ok(d) = d {
                        let publish_packet = PublishPacket::new(TopicName::new(d.topic).unwrap(), QoSWithPacketIdentifier::Level1(10), d.msg.clone());
                        let mut buf = Vec::new();
                        publish_packet.encode(&mut buf).unwrap();
                        stream2.write_all(&buf[..]).unwrap();
                        pkid += 1;
                        if pkid > 65535 {
                            pkid = 100;
                        }
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
    let mut sender = event::init(tx.clone());
    loop {
        let mut sender = sender.clone();
        let packet = match VariablePacket::decode(&mut stream) {
            Ok(pk) => pk,
            Err(err) => {
                error!("Error in receiving packet {}", err);
                continue;
            }
        };
        trace!("PACKET {:?}", packet);

        match packet {
            VariablePacket::PingrespPacket(..) => {
            }
            VariablePacket::PublishPacket(ref publ) => {
                let msg = match str::from_utf8(&publ.payload_ref()[..]) {
                    Ok(msg) => msg,
                    Err(err) => {
                        error!("Failed to decode publish message {:?}", err);
                        continue;
                    }
                };
                let vo : Result<Value> = serde_json::from_str(msg);
                
                if let Ok(v) = vo {
                    if relogin.is_match(publ.topic_name()) {
                        let cap = relogin.captures(publ.topic_name()).unwrap();
                        let userid = cap[1].to_string();
                        info!("login: userid: {} json: {:?}", userid, v);
                        event::login(userid, v, sender.clone())?;
                    }
                    else if relogout.is_match(publ.topic_name()) {
                        let cap = relogin.captures(publ.topic_name()).unwrap();
                        let userid = cap[1].to_string();
                        info!("logout: userid: {} json: {:?}", userid, v);
                        event::logout(userid, v, sender.clone())?;
                    }
                    else if recreate.is_match(publ.topic_name()) {
                        let cap = recreate.captures(publ.topic_name()).unwrap();
                        let userid = cap[1].to_string();
                        info!("create: userid: {} json: {:?}", userid, v);
                        event::create(userid, v, sender.clone())?;
                    }
                    else if reclose.is_match(publ.topic_name()) {
                        let cap = reclose.captures(publ.topic_name()).unwrap();
                        let userid = cap[1].to_string();
                        info!("close: userid: {} json: {:?}", userid, v);
                        event::close(userid, v, sender.clone())?;
                    }
                    else if restart_queue.is_match(publ.topic_name()) {
                        let cap = restart_queue.captures(publ.topic_name()).unwrap();
                        let userid = cap[1].to_string();
                        info!("start_queue: userid: {} json: {:?}", userid, v);
                        event::start_queue(userid, v, sender.clone())?;
                    }
                } else {
                    warn!("Json Parser error");
                };
                
            }
            _ => {}
        }
    }
}
