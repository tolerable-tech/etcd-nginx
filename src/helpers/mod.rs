extern crate rustc_serialize;
use std::io::prelude::*;
use std::fs::File;
use std::path::Path;

use crypto::md5::Md5;
use crypto::digest::Digest;

use std::error::Error;

use mustache;

//use conf_manager::TemplateData;
//use rustc_serialize::serialize::Encodable;
//use rustc_serialize::Encodable;

pub fn sha_str(sstr: &String) -> String {
    let mut generated_hash = Md5::new();
    generated_hash.input(sstr.as_bytes());
    let mut output = [0; 16]; // An MD5 is 16 bytes
    generated_hash.result(&mut output);
    let mut new_sha_str = String::new();
    for dig in output.iter() { new_sha_str.push_str(&dig.to_string()); }
    new_sha_str
}

pub fn open_template(template_name: &str) -> mustache::Template {
    let path = Path::new(template_name);
    let mut f = match File::open(&path) {
        Err(why) => panic!("Couldn't open template file, {}", why.description()),
        Ok(file) => file,
    };

    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();
    let template = mustache::compile_str(&s.to_string());
    template
}

pub fn template_to_string<T, K>(template_name: &str, data: K) -> String where K: rustc_serialize::Encodable {
    let mut bytes = vec![];
    let template = open_template(template_name);

    template.render(&mut bytes, &data).unwrap();
    let template_out_string = String::from_utf8(bytes).unwrap();// {
    template_out_string
}

