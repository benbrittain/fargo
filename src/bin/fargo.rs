#![recursion_limit = "1024"]

extern crate fargo;

use fargo::run;

fn main() {
    if let Err(ref e) = run() {
        println!("error: {}", e);
        ::std::process::exit(1);
    }
}
