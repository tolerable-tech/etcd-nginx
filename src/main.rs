#[macro_use] extern crate lazy_static;
extern crate etcd;
extern crate getopts;
extern crate chrono;

extern crate regex;
extern crate mustache;
extern crate crypto;

extern crate conf_manager;

// opt parsing
use getopts::{Options, Matches};
use std::env;

//use std::thread;
use chrono::*;

//use std::io;
//use mustache::MapBuilder;
//use std::collections::HashMap;
use etcd::Client;
use std::error::Error;

pub mod helpers;
pub mod component_conf;
pub mod ssl_conf;
pub mod static_conf;
pub mod nginx;

use conf_manager::*;

fn etcd_client() -> Client {
    let host = match env::var("HOST_IP") {
        Ok(val) => format!("http://{}:2379", val),
        Err(_) => String::from("http://127.0.0.1:2379"),
    };
    let client = match Client::new(&[&host]) {
        Ok(client) => client,
        Err(why) => panic!("Couldn't get an etcd client: {}", why.description()),
    };
    client
}

fn setup(opts: Matches) -> (Client, ServerState) {
    let client = etcd_client();

    let cps = match opts.opt_str("p") {
        Some(s) => s,
        None => String::new()
    };

    let mut server_state = ServerState::new(cps, &client);
    server_state.loud = opts.opt_present("l");


    (client, server_state)
}

fn loop_body(client: &Client, server_state: &mut ServerState) {
    if ssl_conf::gen(&client, server_state) {
        component_conf::gen(&client, server_state);
    } else {
        server_state.debug("failed to generate ssl certs");
    }

    if server_state.nginx_outdated {
        nginx::reload_conf(server_state);
        server_state.nginx_outdated = false;
    }
}

fn run_components(opts: Matches) {
    let client: Client;
    let mut server_state: ServerState;
    match setup(opts) { 
        (c,s) => {
            client = c;
            server_state = s;
        }
    }

    component_conf::gen(&client, &mut server_state);
    server_state.loud(&format!("{:?}", server_state.existing_apps_conf_sha));
}

fn run_ssl(opts: Matches) {
    let client: Client;
    let mut server_state: ServerState;
    match setup(opts) { 
        (c,s) => {
            client = c;
            server_state = s;
        }
    }

    ssl_conf::gen(&client, &mut server_state);
}

fn run_ssl_gen(opts: Matches) {
    let client: Client;
    let mut server_state: ServerState;
    match setup(opts) { 
        (c,s) => {
            client = c;
            server_state = s;
        }
    }

    ssl_conf::gen_conf(&client, &mut server_state);
}

fn run_loop(opts: Matches) {
    let cps = match opts.opt_str("p") {
        Some(s) => s,
        None => String::new()
    };
    let client = etcd_client();
    let mut server_state = ServerState::new(cps, &client);
    let mut ticks = 0;

    //println!("ssl state is : {:?}", server_state.ssl_state.certs_state);

    nginx::ensure_running(&server_state);

    loop {
        if ticks == 5 {
            ticks = 0;
            loop_body(&client, &mut server_state);
        } else {
            ticks = ticks + 1;
            //println!("waiting...");
        }
        std::thread::sleep(Duration::seconds(1).to_std().unwrap());
    }
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} FILE [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt("p", "cert-path", "the path to put ssl certs.", "PATH");
    opts.optflag("c", "comps", "only generate component confs.");
    opts.optflag("s", "ssl", "only generate ssl confs.");
    opts.optflag("f", "fetch-ssl", "generate ssl confs and run le-fetch.");
    opts.optflag("l", "loud", "send confs to STDOUT.");
    opts.optflag("h", "help", "print this help menu.");
    opts.optflag("v", "version", "print version information.");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(f) => { 
            println!("invalid option received: {}", f.to_string());
            print_usage(&program, opts);
            return;
        }
    };
    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }
    if matches.opt_present("v") {
        println!("conf_manager: v0.1.2");
        return;
    }
    //let output = matches.opt_str("p");
    //let input = if !matches.free.is_empty() {
        //matches.free[0].clone()
    //} else {
        //print_usage(&program, opts);
        //return;
    //};

    if matches.opt_present("c") {
        run_components(matches);
        return;
    }

    if matches.opt_present("s") {
        run_ssl_gen(matches);
        return;
    }

    if matches.opt_present("f") {
        run_ssl(matches);
        return;
    }

    run_loop(matches);
}

