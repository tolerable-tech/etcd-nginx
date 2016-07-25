
extern crate etcd;
extern crate mustache;
extern crate crypto;
extern crate getopts;
extern crate chrono;
extern crate rustc_serialize;

//use crypto::md5::Md5;
//use crypto::digest::Digest;

// opt parsing
//use getopts::{Options, Matches};
//use std::env;

//use std::thread;
use chrono::*;

//use std::io;
//use mustache::MapBuilder;
//use std::collections::HashMap;

//#[macro_use] extern crate lazy_static;
//extern crate regex;
//use regex::Regex;

use etcd::Client;

use std::error::Error;
//use std::io::prelude::*;
//use std::fs::File;
use std::path::Path;

#[derive(RustcEncodable, Debug)]
pub enum SSLCertState {
    Failed,
    Fetching,
    Pending,
    Expired,
    Available,
}
#[derive(RustcEncodable, Debug)]
pub struct SSLState {
    pub enabled: bool,
    pub certs_state: SSLCertState,
    pub requested_certs: Vec<String>,
    pub generated_at: String,
    pub cert_path: String,
    pub le_staging: bool,
}
impl SSLState {
    pub fn new(cert_string: String, client: &Client) -> SSLState {
        let enabled = SSLState::fetch_le_enabled(client);
        let gen_at: String;
        let state:  SSLCertState;
        let mut le_staging = true;
        let domains = SSLState::fetch_requested_certs(client);

        if enabled {
            let cert_path = if cert_string.len() == 0 {
                if domains.len() == 0 {
                    panic!("No domains set for ssl cert, please set 'ssl/domains' to enable SSL.");
                } else {
                    //println!("domains => {}", domains[0]);
                    Path::new("/asdf")
                    //Path::new(format!("/etc/acme/{}/cert.cer", &domains[0]))
                }
            } else { Path::new(&cert_string) };

            if cert_path.is_file() {
                // fetch generated at, Available
                gen_at = match client.get("/ssl/generatedat", false, false, false) {
                    Ok(val) => val.node.unwrap().value.unwrap(),
                    Err(_wut) => String::from("923456789"),
                };
                state = SSLCertState::Available;
            } else {
                // blank generated at, Pending
                gen_at = String::from("923456789");
                state = SSLCertState::Pending;
            }

            le_staging = SSLState::fetch_le_staging(client);
        } else {
            // blank generated at, requested certs, Pending
            gen_at = String::from("923456789");
            state = SSLCertState::Pending;
        }

        SSLState { cert_path: cert_string, enabled: enabled, certs_state: state,
          requested_certs: domains, generated_at: gen_at, le_staging: le_staging,}
    }

    pub fn expired(&self) -> bool {
        let gen_at = match UTC.datetime_from_str(&self.generated_at, "%s") {
            Ok(res) => res,
            Err(_) => UTC.datetime_from_str("994518299", "%s").unwrap(),
        };

        gen_at > gen_at + Duration::days(59)
    }
    
    pub fn refresh(&mut self, client: &Client) {
        self.requested_certs = SSLState::fetch_requested_certs(client);
        self.enabled = SSLState::fetch_le_enabled(client);
        self.le_staging = SSLState::fetch_le_staging(client);
        if self.expired() { self.certs_state = SSLCertState::Expired };
    }

    fn fetch_requested_certs(client: &Client) -> Vec<String> {
        let domains = match client.get("/ssl/domains", false, false, false) {
            Ok(keyspace) => {
                keyspace.node.unwrap().value.unwrap().split(',').map(|s| String::from(s)).collect()
            },
            Err(_wut) => vec![],
        };
        domains
    }

    fn fetch_le_enabled(client: &Client) -> bool {
        let enabled_str = match client.get("/ssl/enabled", false, false, false) {
            Ok(key) => key.node.unwrap().value.unwrap(),
            Err(_wut) => String::from("false")
        };
        if enabled_str.eq("true") { true } else { false }
    }

    fn fetch_le_staging(client: &Client) -> bool {
        let strs = match client.get("/ssl/staging", false, false, false) {
            Ok(val) => val.node.unwrap().value.unwrap(),
            Err(_wut) => String::from("true"),
        };
        if strs.eq("true") { true } else { false }
    }
}
#[derive(RustcEncodable, Debug)]
pub struct ServerState {
    pub ssl_state: SSLState,
    pub existing_apps_conf_sha: String,

    pub components: Vec<ComponentSpec>,
    pub endpoints: Vec<String>
}
impl ServerState {
    pub fn new(cert_string: String, client: &Client) -> ServerState {
        ServerState {
            ssl_state: SSLState::new(cert_string, client),
            existing_apps_conf_sha: String::from(""),
            components: vec![], endpoints: vec![],
        }
    }

    pub fn ssl_ready(&self) -> bool {
        match self.ssl_state.certs_state {
            SSLCertState::Available => true,
            _ => false
        }
    }

}

#[derive(RustcEncodable, Debug)]
pub struct ComponentSpec {
    pub name: String,
    pub upstreams: Vec<String>,
    pub vhosts: Vec<String>,
    pub endpoint: String,
    pub ssl_redirect: bool,
    pub is_fulcrum: bool,
    pub configured: bool,
}
impl ComponentSpec {
    pub fn new(name: String) -> ComponentSpec {
        ComponentSpec {
            name: name,
            upstreams: vec![],
            vhosts: vec![],
            endpoint: String::new(),
            ssl_redirect: false,
            is_fulcrum: false,
            configured: false,
        }
    }

    pub fn finalize(&mut self) {
        if self.vhosts.len() != 0 && self.upstreams.len() != 0 {
            self.configured = true;
        }
    }
}

#[derive(RustcEncodable, Debug)]
pub struct MustacheHolder {
    pub endpoints: Vec<String>,
    pub components: Vec<ComponentSpec>,
    pub ssl_ready: bool
}
impl MustacheHolder {
    pub fn new(comps: Vec<ComponentSpec>, endpoints: Vec<String>, ssl_ready: bool) -> MustacheHolder {
        MustacheHolder {
            endpoints: endpoints, components: comps, ssl_ready: ssl_ready,
        }
    }
}

#[derive(RustcEncodable, Debug)]
pub struct SSLMustacheHolder {
    pub maindomain: String,
    pub others: String,
    pub le_stage: bool
}
impl SSLMustacheHolder {
    pub fn new(maindomain: &str, others: String, le_stage: bool) -> SSLMustacheHolder {
        SSLMustacheHolder {
            maindomain: String::from(maindomain), others: String::from(others), le_stage: le_stage,
        }
    }
}

