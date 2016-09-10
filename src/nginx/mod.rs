use std::process::Command;
use std::error::Error;

use helpers;
use std::fs;
use conf_manager::ServerState;

const SSL_APPS_CONF: &'static str = "/etc/nginx/conf.d/ssl_apps.conf";
const APPS_CONF: &'static str = "/etc/nginx/conf.d/non_ssl_apps.conf";

pub fn is_running() -> bool {
    if helpers::is_file("/var/run/nginx.pid") {
        let pid = helpers::read_file("/var/run/nginx.pid");
        match Command::new("ps").args(&["-p", pid.as_str()]).output() {
            Ok(_) => true,
            Err(_) => false,
        }
    } else {
        false
    }
}

pub fn ensure_running(server_state: &ServerState) {
    if !is_running( ){
        server_state.info("Nginx was not running running.");
        if !test_confs() {
            server_state.info("Nginx are invalid, removing ang regenning.");
            fs::remove_file(SSL_APPS_CONF).expect(&format!("failed to remove {}", SSL_APPS_CONF));
            fs::remove_file(APPS_CONF).expect(&format!("failed to remove {}", SSL_APPS_CONF));
        }
        start(server_state);
    }
}

pub fn start(server_state: &ServerState) {
    match Command::new("nginx").spawn() {
        Ok(_) => server_state.info("nginx server started."),
        Err(w) => panic!("Failed to start nginx server: {}", w.description()),
    };
}

pub fn reload_conf(server_state: &ServerState) {
    match Command::new("nginx").args(&["-s", "reload"]).output() {
        Ok(_) => server_state.debug("nginx reloaded!"),
        Err(w) => panic!("Failed to reload nginx confs: {}", w.description()),
    };
}

pub fn test_confs() -> bool {
    match Command::new("nginx").args(&["-t"]).output() {
        Ok(o) => {
            println!("stdout: {}", &String::from_utf8_lossy(&o.stdout));
            o.status.success()
        },
        Err(_) => false,
    }
}

