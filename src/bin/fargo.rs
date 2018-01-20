#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;
extern crate fargo;

use fargo::run;

error_chain!{}

fn main() {
    if let Err(ref e) = run() {
<<<<<<< Updated upstream
        println!("error: {}", e);

        for e in e.iter().skip(1) {
            println!("caused by: {}", e);
        }

        // The backtrace is not always generated. Try to run this example
        // with `RUST_BACKTRACE=1`.
        if let Some(backtrace) = e.backtrace() {
            println!("backtrace: {:?}", backtrace);
        }

=======
        let causes_string: Vec<String> = e.causes().map(|cause| cause.to_string()).collect();
        println!("error: {}", causes_string.join(", "));
>>>>>>> Stashed changes
        ::std::process::exit(1);
    }
}
