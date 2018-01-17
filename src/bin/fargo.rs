#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;
extern crate fargo;

use fargo::run;

error_chain!{}

fn main() {
    if let Err(ref e) = run() {
        let causes_string: Vec<String> = e.causes().map(|cause| cause.to_string()).collect();
        println!("error: {}", causes_string.join(", "));
        ::std::process::exit(1);
    }
}
