
extern crate etcd;
extern crate mustache;
extern crate crypto;
extern crate getopts;
extern crate chrono;
extern crate rustc_serialize;

use chrono::*;

use etcd::Client;

use std::path::Path;
use std::env;

use std::cmp::Ordering;

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
        let defaulted_cert_string: String;
        let domains = SSLState::fetch_requested_certs(client);

        if enabled {
            let cert_path = if cert_string.len() == 0 {
                if domains.len() == 0 {
                    panic!("No domains set for ssl cert, please set 'ssl/domains' to enable SSL.");
                } else {
                    defaulted_cert_string = format!("/etc/acme/{}/cert.cer", domains[0]);
                    Path::new(&defaulted_cert_string)
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

    pub fn certs_generated(&mut self, client: &Client) {
        let utc: DateTime<UTC> = UTC::now(); 
        let gatstamp = utc.format("%s").to_string();
        client.set("/ssl/generatedat", &gatstamp, None).unwrap();
        self.generated_at = gatstamp;
        self.certs_state = SSLCertState::Available;
    }

    pub fn fetching_certs(&mut self) {
        self.certs_state = SSLCertState::Fetching;
    }

    pub fn fetch_failed(&mut self) {
        self.certs_state = SSLCertState::Failed;
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

    pub nginx_outdated: bool,

    pub components: Vec<ComponentSpec>,
    pub endpoints: Vec<String>,

    pub debug: bool,
    pub loud: bool,
}
impl ServerState {
    pub fn new(cert_string: String, client: &Client) -> ServerState {
        let debug = match env::var("DEBUG") {
            Ok(_) => true,
            Err(_) => false,
        };

        ServerState {
            ssl_state: SSLState::new(cert_string, client), debug: debug,
            existing_apps_conf_sha: String::from(""), loud: false,
            components: vec![], endpoints: vec![], nginx_outdated: false,
        }
    }

    pub fn ssl_ready(&self) -> bool {
        match self.ssl_state.certs_state {
            SSLCertState::Available => true,
            _ => false
        }
    }

    pub fn debug(&self, str: &str) {
        if self.debug {
            println!("{}", str);
        }
    }

    pub fn loud(&self, str: &str) {
        if self.loud {
            println!("{}", str);
        }
    }

    pub fn info(&self, str: &str) {
        println!("{}", str);
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
    pub name_list: String,
    pub show_placeholder: bool,
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
            name_list: String::new(),
            show_placeholder: false,
        }
    }

    pub fn finalize(&mut self, server_state: &ServerState) {
        if self.vhosts.len() != 0 && self.upstreams.len() != 0 {
            self.upstreams.sort();
            self.configured = true;
            self.name_list = self.vhosts.join(", ");
        }
        if self.ssl_redirect && !server_state.ssl_ready() {
            self.show_placeholder = true;
        }
    }
}
impl PartialEq for ComponentSpec  {
    fn eq(&self, other: &ComponentSpec) -> bool {
        self.name.eq(&other.name)
    }
}
impl Eq for ComponentSpec {}
impl Ord for ComponentSpec {
    fn cmp(&self, other: &ComponentSpec) -> Ordering {
        self.name.cmp(&other.name)
    }
}
impl PartialOrd for ComponentSpec {
    fn partial_cmp(&self, other: &ComponentSpec) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(RustcEncodable, Debug)]
pub struct StaticSpec {
    pub name: String,
    pub vhosts: Vec<String>,
    pub path: String,
    pub is_fulcrum: bool,
    pub configured: bool,
    pub name_list: String,
    pub force_ssl: bool,
    pub ssl_enabled: bool,
}
impl StaticSpec {
    pub fn new(name: String) -> StaticSpec {
        StaticSpec {
            name: name,
            vhosts: vec![],
            path: String::new(),
            is_fulcrum: false,
            configured: false,
            name_list: String::new(),
            force_ssl: true,
            ssl_enabled: false,
        }
    }

    pub fn finalize(&mut self, _server_state: &ServerState) {
        if self.vhosts.len() != 0 {
            self.configured = true;
            self.name_list = self.vhosts.join(", ");
        }
    }
}
impl PartialEq for StaticSpec  {
    fn eq(&self, other: &StaticSpec) -> bool {
        self.name.eq(&other.name)
    }
}
impl Eq for StaticSpec {}
impl Ord for StaticSpec {
    fn cmp(&self, other: &StaticSpec) -> Ordering {
        self.name.cmp(&other.name)
    }
}
impl PartialOrd for StaticSpec {
    fn partial_cmp(&self, other: &StaticSpec) -> Option<Ordering> {
        Some(self.cmp(other))
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
    pub domains: String,
    pub le_stage: bool
}
impl SSLMustacheHolder {
    pub fn new(domain: &str, le_stage: bool) -> SSLMustacheHolder {
        SSLMustacheHolder {
            domains: String::from(domain), le_stage: le_stage,
        }
    }
}

#[derive(RustcEncodable, Debug)]
pub struct StaticMustacheHolder {
    pub folders: Vec<StaticSpec>,
    pub ssl_ready: bool
}
impl StaticMustacheHolder {
    pub fn new(folders: Vec<StaticSpec>, ssl: bool) -> StaticMustacheHolder {
        StaticMustacheHolder {
            folders: folders, ssl_ready: ssl,
        }
    }
}

