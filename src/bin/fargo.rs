#![recursion_limit = "1024"]

extern crate failure;
extern crate fargo;

use fargo::run;

fn main() {
    if let Err(ref e) = run() {
        let causes_string: Vec<String> = e.causes().map(|cause| cause.to_string()).collect();
        println!("error: {}", causes_string.join(", "));
        ::std::process::exit(1);
    }
}
