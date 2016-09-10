extern crate etcd;

use std::path::Path;
use std::fs;
use std::os::unix;
use std::error::Error;

use std::process::{Command, Output};

//use chrono::*;

use etcd::Client;

use conf_manager::*;
use helpers;
use nginx;

use static_conf;

// check SSL
//   - check if certs exist
//   - check if certs expired
// resolve SSL if necessary
//   - place ssl fetching conf
//   - trigger nginx reload
//   - generate ssl-fetch.conf
//   - run ssl-fetch
pub fn gen(client: &Client, server_state: &mut ServerState) -> bool {
    server_state.ssl_state.refresh(client);
    if server_state.ssl_state.enabled && server_state.ssl_state.requested_certs.len() > 0 {
        let dmvalconf = Path::new("/etc/nginx/conf.d/domain-validation.conf");
        if dmvalconf.is_file() { 
            fs::rename(dmvalconf, Path::new("/etc/nginx/conf.d/domain-validation.off")).unwrap()
        }
        match server_state.ssl_state.certs_state {
            SSLCertState::Available => {
                //server_state.debug("not genning ssl certs: SSLCertState::Available");
                true
            },
            SSLCertState::Fetching => {
                server_state.debug("this will never happen #famouslastwords (SSLCertStateFetching)");
                false
            },
            _ => fetch_certs(client, server_state),
        }
    } else {
        server_state.debug(
            &format!(" -- either ssl is disabled or no certs requested: {} :: {:?}",
                     server_state.ssl_state.enabled, server_state.ssl_state.requested_certs));
        true
    }
}

pub fn gen_conf(_client: &Client, server_state: &mut ServerState) {
    write_ssl_cert_config(server_state);
}

fn fetch_certs(client: &Client, server_state: &mut ServerState) -> bool {
    write_ssl_cert_config(server_state);
    show_placeholder_and_enable_ssl_conf(server_state);

    let ret_bool = if run_le_fetch(server_state) {
        server_state.debug("  -- run_le_fetch returned truthy");
        server_state.ssl_state.certs_generated(client);
        true 
    } else { 
        server_state.debug("  -- run_le_fetch returned falsey");
        server_state.ssl_state.fetch_failed();
        false };
    disable_placeholder(server_state);
    link_main_domain_to_fulcrum_folder(server_state);
    ret_bool
}

fn write_ssl_cert_config(server_state: &mut ServerState) -> bool {
    let mut domains = String::new();

    for o in &server_state.ssl_state.requested_certs {
        domains.push_str(&format!(" -d {} ", o));
    }

    let data = SSLMustacheHolder::new(&domains, true);
    let template_out_string = helpers::template_to_string::<String, SSLMustacheHolder>("ssl", data);

    server_state.loud(&template_out_string);

    helpers::conditionally_update_file("/usr/local/bin/le-fetch.conf",
                                       template_out_string, server_state)
}

fn show_placeholder_and_enable_ssl_conf(server_state: &mut ServerState) {
    //server_state.debug("  - enabling le-auth.conf");
    fs::copy(Path::new("/etc/nginx/conf.d/le-auth.off"),
      Path::new("/etc/nginx/conf.d/aaa-le-auth.conf")).expect("could not enable le-auth.conf");

    let ssl_apps_path = Path::new("/etc/nginx/conf.d/ssl_apps.conf");
    if ssl_apps_path.is_file() {
        fs::rename(ssl_apps_path, Path::new("/etc/nginx/conf.d/ssl_apps.off")
          ).expect("could not disable ssl_apps.conf");
    }

    let static_folders = Path::new(static_conf::SSL_STATIC_CONF);
    if static_folders.is_file() {
        fs::rename(static_folders, static_folders.with_extension("off"))
            .expect("could not disable ssl_static_folders.conf");
    }

    server_state.ssl_state.fetching_certs();
    server_state.debug("  - enabled placeholder, regenerating comp conf and reloading nginx conf");

    nginx::reload_conf(server_state);
}

fn link_main_domain_to_fulcrum_folder(server_state: &ServerState) {
    let f_path = Path::new("/etc/acme/fulcrum");

    if !f_path.is_dir() {
        let name = format!("/etc/acme/{}", server_state.ssl_state.requested_certs[0]);
        let td_path = Path::new(&name);
        unix::fs::symlink(td_path, f_path).expect(
            &format!("failed to symlink {:?} to /etc/acme/fulcrum", td_path));
    }
}

fn disable_placeholder(server_state: &mut ServerState) {
    server_state.debug("  - disabling le-auth.conf");
    fs::remove_file(Path::new("/etc/nginx/conf.d/aaa-le-auth.conf")).unwrap();

    let ssl_apps_path = Path::new("/etc/nginx/conf.d/ssl_apps.off");
    if ssl_apps_path.is_file() {
        fs::rename(ssl_apps_path, Path::new("/etc/nginx/conf.d/ssl_apps.conf")
          ).expect("could not re-enabled ssl_apps.conf");
    }

    let static_folders = Path::new(static_conf::SSL_STATIC_CONF);
    if static_folders.is_file() {
        fs::rename(static_folders, static_folders.with_extension("conf"))
            .expect("could not re-enable ssl_static_folders.conf");
    }

    server_state.debug("  - disabled placeholder and le-auth, reloading nginx conf");

    server_state.nginx_outdated = true;
}

fn run_le_fetch(server_state: &mut ServerState) -> bool {
    let code: i32;
    let output = match Command::new("/usr/local/bin/le-fetch").output() {
        Ok(childe) => {
            match childe.status.code() {
                Some(c) => code = c,
                None => code = 0,
            }
            if  code == 2 { set_expire_date(server_state, &childe) };
            childe
        },
        Err(w) => panic!("Failed to gen certs: {}", w.description()),
    };

    server_state.debug(&format!("stdout: {}", String::from_utf8_lossy(&output.stdout)));
    server_state.debug(&format!("stderr: {}", String::from_utf8_lossy(&output.stderr)));

    if output.status.success() { true } else {
        if code == 2 { true } else { false }
    }
}

fn set_expire_date(_server_state: &mut ServerState, _out: &Output) {
}

