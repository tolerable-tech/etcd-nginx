

extern crate etcd;
//use crypto::md5::Md5;
//use crypto::digest::Digest;

use regex::Regex;

use etcd::Client;
//
use conf_manager::*;
use helpers;
use nginx;

static STATIC_CONF: &'static str = "/etc/nginx/conf.d/static_folders.conf";
pub static SSL_STATIC_CONF: &'static str = "/etc/nginx/conf.d/ssl_static_folders.conf";

fn get_static_spec(key_node: &etcd::Node, client: &Client, server_state: &ServerState) -> StaticSpec {
    lazy_static! {
        static ref VHOST_RE: Regex = Regex::new("/vhost").unwrap();
        static ref SSLRE: Regex = Regex::new("/ssl").unwrap();
        //static ref ENDPOINT_RE: Regex = Regex::new("/endpoint").unwrap();
        static ref PATH_RE: Regex = Regex::new("/path").unwrap();
    }

    let static_key = key_node.key.clone().unwrap();
    let v: Vec<&str> = static_key.split('/').collect();
    let name = v[2];

    let mut cspec = StaticSpec::new(name.to_string());

    if name == "fulcrum" { cspec.is_fulcrum = true };

    let attrs = client.get(&static_key, false, true, false)
        .ok().unwrap().node.unwrap().nodes.unwrap();

    for thing in attrs.iter() {
        let owned_thing = thing.clone();
        match thing {
            vhosts if VHOST_RE.is_match(&owned_thing.key.clone().unwrap()) => {
                //println!("fart! {:?}", vhosts);
                match vhosts.nodes.to_owned() {
                    Some(hosts) => {
                        for vhost in hosts.iter() {
                            //println!("adding vhost!!! {:?}", vhost);
                            cspec.vhosts.push(vhost.value.to_owned().unwrap());
                        };
                        ()
                    },
                    None => (),
                }
            },
            ssl if SSLRE.is_match(&owned_thing.key.clone().unwrap()) => {
                cspec.ssl_enabled = true;
                match ssl.value {
                    Some(ref ss) => {
                        if ss.eq("redirect") { cspec.force_ssl = true }
                    },
                    None => cspec.force_ssl = true,
                }
            }
            path if PATH_RE.is_match(&owned_thing.key.unwrap()) => {
                server_state.debug(&format!("setting path for folder:{} - {:?}", name, path));
                match path.value {
                    Some(ref ep) => cspec.path = ep.clone(),
                    None => (),
                }
            }
            _ => (),
        }
    }

    cspec.finalize(server_state);
    //server_state.debug(&format!("generating component spec for: {}. Configured: {}", name, cspec.configured));
    cspec
}

pub fn gen(client: &Client, server_state: &mut ServerState) {
    let statics = match client.get("/statics", false, false, false) {
        Ok(ksp) => {
            match ksp.node.unwrap().nodes {
                Some(n) => n,
                None => vec![],
        }},
        Err(_) => vec![]
    };

    let mut static_specs: Vec<StaticSpec> = Vec::new();
    let mut ssl_static_specs: Vec<StaticSpec> = Vec::new();
    for statc in statics.iter() {
        let struct_data = get_static_spec(statc, client, server_state);

        if struct_data.force_ssl || struct_data.ssl_enabled {
            ssl_static_specs.push(struct_data);
        } else {
            static_specs.push(struct_data);
        }
    }

    let static_data = StaticMustacheHolder::new(static_specs, false);
    let static_template_string = helpers::template_to_string::<String, StaticMustacheHolder>("static_folders", static_data);

    server_state.loud("  -  static_folder.conf!");
    server_state.loud(&static_template_string);

    let ssl_static_data = StaticMustacheHolder::new(ssl_static_specs, server_state.ssl_ready());
    let ssl_static_template_string = helpers::template_to_string::<String, StaticMustacheHolder>("ssl_static_folders", ssl_static_data);

    server_state.loud("  -  static_folder.conf!");
    server_state.loud(&static_template_string);

    server_state.loud("  -  ssl_static_folder.conf!");
    server_state.loud(&ssl_static_template_string);

    if helpers::conditionally_update_file(
        STATIC_CONF, static_template_string, server_state) {
        server_state.nginx_outdated = true;
    }

    if helpers::conditionally_update_file(
        SSL_STATIC_CONF, ssl_static_template_string, server_state) {
        if nginx::test_confs() {
            server_state.nginx_outdated = true;
        } else {
            server_state.debug("  -- new ssl static folder conf failed to test, rolling back.");
            helpers::rollback_conf(SSL_STATIC_CONF);
        }
    }
    //if !new_sha.eq(&server_state.existing_apps_conf_sha) {
        //server_state.debug(&format!("writing new components conf file....{} == {}",
                                      //new_sha, server_state.existing_apps_conf_sha));
        //helpers::update_file(SSL_APPS_CONF, ssl_template_string);
        //helpers::update_file(APPS_CONF, non_ssl_template_string);
        //if nginx::test_confs() {
            //server_state.existing_apps_conf_sha = new_sha;
            //server_state.nginx_outdated = true;
        //} else {
            //server_state.debug("  -- new conf failed to test, rolling back.");
            //helpers::rollback_conf(SSL_APPS_CONF);
            //helpers::rollback_conf(APPS_CONF);
        //}
    //}
}

