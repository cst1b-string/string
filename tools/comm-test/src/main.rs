use std::{net::IpAddr, net::Ipv4Addr, net::SocketAddr, time::Duration};
use comm::{peer::PeerState, Socket};
use std::env;

#[tokio::main]
async fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() != 5 {
        println!("Usage: {0} srcport dstip dstport <true/false>", args[0]);
    }
    else {
        let peer_addr = SocketAddr::new(
            IpAddr::V4(args[2].parse::<Ipv4Addr>().expect("Bad IP")),
            args[3].parse::<u16>().expect("Bad dstport")
        );
        let sock_addr = SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            args[1].parse::<u16>().expect("Bad srcport")
        );
        let mut sock = match Socket::bind(sock_addr).await {
            Ok(s) => s,
            Err(_) => {
                eprintln!("[-] Failed to bind to local.");
                return;
            }
        };
        let initiate = args[4].parse::<bool>().expect("Bad initiate");
        let (_app_outbound_tx, _app_inbound_rx) = sock.add_peer(peer_addr, initiate).await;
        println!("[+] Setup success!");
        println!("[*] Attempting transmission...");
        let mut i: u16 = 0;
        let mut ready: bool = false;
        // Wait 5 mins
        while i < 5 * 60 * 2 && !ready {
            match sock.get_peer_state(peer_addr).await {
                None => {},
                Some(s) => {
                    if s == PeerState::Established { ready = true; }
                }
            }
            i += 1;
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        if !ready {
            eprintln!("[-] Connection failure.");
            return;
        }
        println!("[+] Connection with {0} succeeded!", peer_addr);
    }
}
