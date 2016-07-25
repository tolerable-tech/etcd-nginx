

extern crate etcd;
//use crypto::md5::Md5;
//use crypto::digest::Digest;

use regex::Regex;
//use chrono::*;

use std::fs::File;
use std::io::Write;
use std::error::Error;

use etcd::Client;
//
use conf_manager::*;
use helpers;

fn get_component_spec(key_node: &etcd::Node, client: &Client) -> ComponentSpec {
    lazy_static! {
        static ref VHOST_RE: Regex = Regex::new("/vhost").unwrap();
        static ref SSLRE: Regex = Regex::new("/ssl").unwrap();
        static ref ENDPOINT_RE: Regex = Regex::new("/endpoint").unwrap();
    }

    let component_key = key_node.key.clone().unwrap();
    let v: Vec<&str> = component_key.split('/').collect();
    let name = v[2];
    //println!("name:: {}", name);

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
            _ssl if SSLRE.is_match(&owned_thing.key.clone().unwrap()) => cspec.ssl_redirect = true,
            endpoint if ENDPOINT_RE.is_match(&owned_thing.key.unwrap()) => {
                println!("{:?}", endpoint);
                match endpoint.value {
                    Some(ref ep) => cspec.endpoint = ep.clone(),
                    None => (),
                }
            }
            upstream => 
                cspec.upstreams.push(upstream.value.to_owned().unwrap()),
        }
    }

    cspec.finalize();
    cspec
}

pub fn gen(client: &Client, server_state: &ServerState) -> String {
    let components = match client.get("/apps", false, false, false) {
        Ok(ksp) => ksp.node.unwrap().nodes.unwrap(),
        Err(_) => vec![]
    };

    let mut comp_vecs: Vec<ComponentSpec> = Vec::new();
    let mut endpoints: Vec<String> = Vec::new();
    for component in components.iter() {
        //println!("yay! {:?}", component);
        let struct_data = get_component_spec(component, client);
        endpoints.push(struct_data.endpoint.clone());
        comp_vecs.push(struct_data);
    }

    let data = MustacheHolder::new(comp_vecs, endpoints, server_state.ssl_ready()); // { endpoints: endpoints, components: comp_vecs};
    let template_out_string = helpers::template_to_string::<String, MustacheHolder>("apps.mustache", data);

    let new_sha_str = helpers::sha_str(&template_out_string);

    //println!("{} == {} #=> {}", server_state.existing_apps_conf_sha, new_sha_str, new_sha_str.eq(&server_state.existing_apps_conf_sha));

    if !new_sha_str.eq(&server_state.existing_apps_conf_sha) {
        println!("writing new components conf file....");
        let mut outfile = match File::create("testout.conf"){
            Err(why) => panic!("Couldn't open template file, {}", why.description()),
            Ok(file) => file,
        };
        outfile.write_all(template_out_string.as_bytes()).unwrap();
    }

    new_sha_str
}

