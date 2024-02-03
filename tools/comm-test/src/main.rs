use comm::{Connection, ConnState};
use std::env;

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() != 5 {
        println!("Bad args");
    }
    else {
        let mut conn = Connection::new(
            &args[1],
            args[2].parse::<u16>().unwrap(),
            args[3].parse::<u16>().unwrap(),
            args[4].parse::<bool>().unwrap(),
        );
        let _ = conn.connect();
        match conn.state {
            ConnState::CONNECTED => println!("Connected"),
            ConnState::DISCON => println!("Not connected")
        }
    }
}
