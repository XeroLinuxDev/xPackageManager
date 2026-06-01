// mimic the UI AppConfig subset to test serde tolerance
use serde::Deserialize;
#[derive(Deserialize,Debug,Default)]
#[serde(default)]
struct C{ appimage_enabled:bool, appimage_dir:String, appimage_feeds:Vec<F> }
#[derive(Deserialize,Debug,Default)] #[serde(default)] struct F{ name:String, url:String }
fn main(){ let s=std::fs::read_to_string("/tmp/stale.json").unwrap();
 match serde_json::from_str::<C>(&s){ Ok(c)=>println!("OK enabled={} feeds={}",c.appimage_enabled,c.appimage_feeds.len()), Err(e)=>println!("PARSE FAIL: {}",e) } }
