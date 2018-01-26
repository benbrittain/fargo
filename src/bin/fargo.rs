#![recursion_limit = "1024"]

extern crate failure;
extern crate fargo;
extern crate itertools;

use fargo::run;
use itertools::Itertools;

fn main() {
    if let Err(ref e) = run() {
        println!("error: {}", e.causes().join(", "));
        ::std::process::exit(1);
    }
}
