extern crate etcd;

use std::fs::File;
use std::io::Write;
use std::error::Error;

use std::process::Command;

use chrono::*;

use etcd::Client;

use conf_manager::*;
use helpers;


// check SSL
//   - check if certs exist
//   - check if certs expired
// resolve SSL if necessary
//   - place ssl fetching conf
//   - trigger nginx reload
//   - generate ssl-fetch.conf
//   - run ssl-fetch
pub fn gen(client: &Client, server_state: &mut ServerState) {
    server_state.ssl_state.refresh(client);
    if server_state.ssl_state.enabled && server_state.ssl_state.requested_certs.len() > 0 {
        match server_state.ssl_state.certs_state {
            SSLCertState::Available => println!("certs are good!"),
            SSLCertState::Fetching => println!("this will never happen #famouslastwords"),
            _ => fetch_certs(client, server_state),
        }
    }
}

fn fetch_certs(client: &Client, server_state: &mut ServerState) {
    //println!("fetching certs!!");
    write_ssl_cert_config(server_state);

    if run_le_fetch() {
        set_updated_generated_at(client);
        server_state.ssl_state.certs_state = SSLCertState::Available;
    }
}

fn write_ssl_cert_config(server_state: &mut ServerState) {
    let main = &server_state.ssl_state.requested_certs[0];
    let mut others = String::new();

    for o in &server_state.ssl_state.requested_certs {
        if !o.eq(main) {
            others.push_str(&format!("{},", o));
        }
    }

    let data = SSLMustacheHolder::new(main, others, true);
    let template_out_string = helpers::template_to_string::<String, SSLMustacheHolder>("ssl.mustache", data);

    let new_sha_str = helpers::sha_str(&template_out_string);

    if !new_sha_str.eq(&server_state.existing_apps_conf_sha) {
        println!("writing new ssl conf file....");
        let mut outfile = match File::create("ssl.conf"){
            Err(why) => panic!("Couldn't open template file, {}", why.description()),
            Ok(file) => file,
        };
        outfile.write_all(template_out_string.as_bytes()).unwrap();
    }
}

fn run_le_fetch() -> bool {
    let output = match Command::new("/usr/local/bin/lefetch").output() {
        Ok(childe) => childe,
        Err(w) => panic!("Failed to gen certs: {}", w.description()),
    };

	//println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
	//println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    output.status.success()
}

fn set_updated_generated_at(client: &Client) {
    let utc: DateTime<UTC> = UTC::now(); 
    let gatstamp = utc.format("%s").to_string();
    client.set("/ssl/generatedat", &gatstamp, None).unwrap();
}

