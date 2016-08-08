

extern crate etcd;
//use crypto::md5::Md5;
//use crypto::digest::Digest;

use regex::Regex;

use etcd::Client;
//
use conf_manager::*;
use helpers;
use nginx;

const SSL_APPS_CONF: &'static str = "/etc/nginx/conf.d/ssl_apps.conf";
const APPS_CONF: &'static str = "/etc/nginx/conf.d/non_ssl_apps.conf";

fn get_component_spec(key_node: &etcd::Node, client: &Client, server_state: &ServerState) -> ComponentSpec {
    lazy_static! {
        static ref VHOST_RE: Regex = Regex::new("/vhost").unwrap();
        static ref SSLRE: Regex = Regex::new("/ssl").unwrap();
        static ref ENDPOINT_RE: Regex = Regex::new("/endpoint").unwrap();
    }

    let component_key = key_node.key.clone().unwrap();
    let v: Vec<&str> = component_key.split('/').collect();
    let name = v[2];

    let mut cspec = ComponentSpec::new(name.to_string());

    if name == "fulcrum" { cspec.is_fulcrum = true };

    let specs = client.get(&component_key, false, true, false)
        .ok().unwrap().node.unwrap().nodes.unwrap();

    for thing in specs.iter() {
        let owned_thing = thing.clone();
        match thing {
            vhosts if VHOST_RE.is_match(&owned_thing.key.clone().unwrap()) => {
                //println!("fart! {:?}", vhosts);
                match vhosts.nodes.to_owned() {
                    Some(mut hosts) => {
                        hosts.sort_by(|a,b| a.key.clone().unwrap().cmp(&b.key.clone().unwrap()));
                        for vhost in hosts.iter() {
                            //println!("adding vhost!!! {:?}", vhost);
                            cspec.vhosts.push(vhost.value.to_owned().unwrap());
                        };
                        ()
                    },
                    None => (),
                }
            },
            _ssl if SSLRE.is_match(&owned_thing.key.clone().unwrap()) => cspec.ssl_redirect = true,
            endpoint if ENDPOINT_RE.is_match(&owned_thing.key.unwrap()) => {
                server_state.debug(&format!("setting endpoint for c:{} - {:?}", name, endpoint));
                match endpoint.value {
                    Some(ref ep) => cspec.endpoint = ep.clone(),
                    None => (),
                }
            }
            upstream => 
                cspec.upstreams.push(upstream.value.to_owned().unwrap()),
        }
    }

    cspec.finalize(server_state);
    //server_state.debug(&format!("generating component spec for: {}. Configured: {}", name, cspec.configured));
    cspec
}

pub fn gen(client: &Client, server_state: &mut ServerState) {
    let components = match client.get("/apps", false, false, false) {
        Ok(ksp) => {
            match ksp.node.unwrap().nodes {
                Some(n) => n,
                None => vec![],
        }},
        Err(_) => vec![]
    };

    let mut ssl_components: Vec<ComponentSpec> = Vec::new();
    let mut non_ssl_components: Vec<ComponentSpec> = Vec::new();
    let mut endpoints: Vec<String> = Vec::new();
    for component in components.iter() {
        let struct_data = get_component_spec(component, client, server_state);
        endpoints.push(struct_data.endpoint.clone());

        if struct_data.ssl_redirect {
            ssl_components.push(struct_data);
        } else {
            non_ssl_components.push(struct_data);
        }
    }

    let ssl_data = MustacheHolder::new(ssl_components, endpoints, server_state.ssl_ready());
    let ssl_template_string = helpers::template_to_string::<String, MustacheHolder>("ssl_apps", ssl_data);

    server_state.loud("  -  ssl_app.conf!");
    server_state.loud(&ssl_template_string);

    let non_ssl_data = MustacheHolder::new(non_ssl_components, vec![], false);
    let non_ssl_template_string = helpers::template_to_string::<String, MustacheHolder>("non_ssl_apps", non_ssl_data);

    server_state.loud("  -  app.conf!");
    server_state.loud(&non_ssl_template_string);

    let ssl_sha = helpers::sha_str(&ssl_template_string);
    let nssl_sha = helpers::sha_str(&non_ssl_template_string);
    let new_sha = format!("{}:{}", ssl_sha, nssl_sha);


    if !new_sha.eq(&server_state.existing_apps_conf_sha) {
        server_state.debug(&format!("writing new components conf file....{} == {}",
                                      new_sha, server_state.existing_apps_conf_sha));
        helpers::update_file(SSL_APPS_CONF, ssl_template_string);
        helpers::update_file(APPS_CONF, non_ssl_template_string);
        if nginx::test_confs() {
            server_state.existing_apps_conf_sha = new_sha;
            server_state.nginx_outdated = true;
        } else {
            server_state.debug("  -- new conf failed to test, rolling back.");
            helpers::rollback_conf(SSL_APPS_CONF);
            helpers::rollback_conf(APPS_CONF);
        }
    }
}

