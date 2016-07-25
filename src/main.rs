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

pub mod helpers;
pub mod component_conf;
pub mod ssl_conf;

use conf_manager::*;

fn loop_body(client: &Client, server_state: &mut ServerState) {
    // check SSL
    //   - check if certs exist
    //   - check if certs expired
    // resolve SSL if necessary
    //   - place ssl fetching conf
    //   - trigger nginx reload
    //   - generate ssl-fetch.conf
    //   - run ssl-fetch
    //   - restart loop
    // generate component comfiguration
    // trigger nginx reload if necessary
    //ssl_check(client, server_state);
    println!("gathering confs....");
    ssl_conf::gen(&client, server_state);
    let sha = component_conf::gen(&client, &server_state);
    server_state.existing_apps_conf_sha = sha;
    println!("{:?}", server_state.existing_apps_conf_sha);
}

fn run_once(opts: Matches) {
    let cps = match opts.opt_str("p") {
        Some(s) => s,
        None => String::new()
    };
    let client = Client::default();
    let mut server_state = ServerState::new(cps, &client);

    let sha = component_conf::gen(&client, &server_state);
    server_state.existing_apps_conf_sha = sha;
    println!("{:?}", server_state.existing_apps_conf_sha);
}

fn run_loop(opts: Matches) {
    let cps = match opts.opt_str("p") {
        Some(s) => s,
        None => String::new()
    };
    let client = Client::default();
    let mut server_state = ServerState::new(cps, &client);
    let mut ticks = 0;

    //println!("ssl state is : {:?}", server_state.ssl_state.certs_state);

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
    opts.optflag("p", "cert-path", "the path to put ssl certs.");
    opts.optflag("c", "comps", "only generate component confs");
    opts.optflag("s", "ssl", "only generate ssl confs");
    opts.optflag("l", "loud", "send confs to STDOUT");
    opts.optflag("h", "help", "print this help menu");
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
    //let output = matches.opt_str("p");
    //let input = if !matches.free.is_empty() {
        //matches.free[0].clone()
    //} else {
        //print_usage(&program, opts);
        //return;
    //};

    if matches.opt_present("c") {
        run_once(matches);
    } else {
        run_loop(matches);
    }

}

