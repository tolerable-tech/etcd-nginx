extern crate rustc_serialize;
use std::io::prelude::*;
use std::fs;
use std::fs::File;
use std::path::Path;

use crypto::md5::Md5;
use crypto::digest::Digest;

use std::error::Error;

use conf_manager::ServerState;

use mustache;

//use conf_manager::TemplateData;
//use rustc_serialize::serialize::Encodable;
//use rustc_serialize::Encodable;

pub fn conditionally_update_file(path_str: &str, contents: String, server_state: &ServerState) -> bool {
    let existing_sha = sha_file(path_str);
    let candidate_sha = sha_str(&contents);

    if existing_sha.eq(&candidate_sha) {
        false
    } else {
        server_state.debug(&format!(" - updating {} to new contents.", path_str));
        server_state.loud("  -- static_folder.conf ");
        server_state.loud(&contents);
        update_file(path_str, contents);
        true
    }
}

pub fn sha_str(sstr: &String) -> String {
    let mut generated_hash = Md5::new();
    generated_hash.input(sstr.as_bytes());
    let mut output = [0; 16]; // An MD5 is 16 bytes
    generated_hash.result(&mut output);
    let mut new_sha_str = String::new();
    for dig in output.iter() { new_sha_str.push_str(&dig.to_string()); }
    new_sha_str
}

pub fn sha_file(path_str: &str) -> String {
    if Path::new(path_str).is_file() {
        let file_contents = read_file(path_str);
        sha_str(&file_contents)
    } else {
        String::new()
    }
}

pub fn update_file(path_str: &str, contents: String) {
    let dest_path = Path::new(path_str);

    if dest_path.is_file() {
        let prev_path = dest_path.with_extension("prev");
        fs::rename(dest_path, prev_path).expect(
            &format!("failed to move {} to .prev", dest_path.to_string_lossy()));
    }

    let mut outfile = match File::create(&dest_path) {
        Err(why) => panic!("Couldn't open {}: {}", dest_path.to_string_lossy(), why.description()),
        Ok(file) => file,
    };
    outfile.write_all(contents.as_bytes()).expect(
        &format!("failed to write updated file contents: {}", dest_path.to_string_lossy()));
}

pub fn is_file(path_str: &str) -> bool {
    Path::new(path_str).is_file()
}

pub fn read_file(path_str: &str) -> String {
    let path = Path::new(&path_str);
    let mut f = match File::open(&path) {
        Err(why) => panic!("Couldn't open file '{}', {}", path_str, why.description()),
        Ok(file) => file,
    };

    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();
    s
}

pub fn open_template(template_name: &str) -> mustache::Template {
    let template_path = format!("/usr/local/lib/conf_templates/{}.mustache", template_name);

    let s = read_file(&template_path);

    mustache::compile_str(&s.to_string())
}

pub fn template_to_string<T, K>(template_name: &str, data: K) -> String where K: rustc_serialize::Encodable {
    let mut bytes = vec![];
    let template = open_template(template_name);

    template.render(&mut bytes, &data).unwrap();
    let template_out_string = String::from_utf8(bytes).unwrap();// {
    template_out_string
}

pub fn rollback_conf(path_str: &str) {
    let path = Path::new(path_str);
    if path.is_file() {
        fs::rename(path, path.with_extension("failing")).expect(
            &format!("unable to rollback conf: {:?}", path));
    }
    
    let prev_pbuff = path.with_extension("prev");
    let prev_path = prev_pbuff.as_path();
    if prev_path.is_file() {
        fs::rename(prev_path, prev_path.with_extension("conf")).expect(
            &format!("unable to restore conf: {:?}", prev_path));
    }
}

