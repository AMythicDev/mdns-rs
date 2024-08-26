use if_addrs::Interface;
use socket2::Socket;
use std::{
    collections::HashMap,
    io,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    thread,
};
use tracing::{info, warn};

const MDNS_PORT: u16 = 5353;
const MDNS_ADDR_V4: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const MDNS_ADDR_V6: Ipv6Addr = Ipv6Addr::new(0xff02, 0, 0, 0, 0, 0, 0, 0xfb);

pub struct PeerDaemon {
    interfaces: HashMap<Interface, Socket>,
}

impl PeerDaemon {
    pub fn new() -> Self {
        info!("starting the mDNS daemon");
        let interfaces = make_interface_socket_map();
        // Self::start_daemon(socket);
        Self { interfaces }
    }

    // fn start_daemon(socket: UdpSocket) {
    //     let _ = thread::Builder::new()
    //         .name("mDNS peer daemon".to_string())
    //         .spawn(move || {
    //             let mut buf = [0; 1024];
    //             loop {
    //                 socket.send_to(b"Hello World", "10.89.0.2").unwrap();
    //                 match socket.recv_from(&mut buf) {
    //                     Ok(n) => break n,
    //                     Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
    //                         // wait until network socket is ready, typically implemented
    //                         // via platform-specific APIs such as epoll or IOCP
    //                         // wait_for_fd();
    //                     }
    //                     Err(e) => panic!("encountered IO error: {e}"),
    //                 }
    //             }
    //         })
    //         .unwrap()
    //         .join();
    // }
}

fn make_interface_socket_map() -> HashMap<Interface, Socket> {
    let mut interfaces = HashMap::new();
    let interface_iter = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|i| !i.is_loopback());

    for interface in interface_iter {
        let ip = &interface.ip();
        let sock = match ip {
            IpAddr::V4(ip) => {
                let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), MDNS_PORT);
                let sock = match udp_socket(addr.into()) {
                    Ok(sock) => sock,
                    Err(e) => {
                        warn!("failed to bind socket for '{}' due to {e}", ip);
                        continue;
                    }
                };

                // Join mDNS group to receive packets.
                if let Err(e) = sock.join_multicast_v4(&MDNS_ADDR_V4, ip) {
                    warn!(
                        "socket with address {} failed to join multicast group due to {e}",
                        ip
                    );
                    continue;
                }

                // Set IP_MULTICAST_IF to send packets.
                if let Err(e) = sock.set_multicast_if_v4(ip) {
                    warn!(
                        "socket with address {} failed to set to multicast  due to {e}",
                        ip
                    );
                    continue;
                }
                let multicast_addr = SocketAddrV4::new(MDNS_ADDR_V4, MDNS_PORT).into();
                sock.send_to(b"Hello World", &multicast_addr).unwrap();
                sock
            }
            IpAddr::V6(ip) => {
                let addr =
                    SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0), MDNS_PORT, 0, 0);
                let sock = match udp_socket(addr.into()) {
                    Ok(sock) => sock,
                    Err(e) => {
                        warn!("failed to bind socket for '{}' due to {e}", ip);
                        continue;
                    }
                };

                // Join mDNS group to receive packets.
                if let Err(e) = sock.join_multicast_v6(&MDNS_ADDR_V6, interface.index.unwrap_or(0))
                {
                    warn!(
                        "socket with address {} failed to join multicast group due to {e}",
                        ip
                    );
                    continue;
                }

                // Set IP_MULTICAST_IF to send packets.
                if let Err(e) = sock.set_multicast_if_v6(interface.index.unwrap_or(0)) {
                    warn!(
                        "socket with address {} failed to set to multicast  due to {e}",
                        ip
                    );
                    continue;
                }
                sock
            }
        };
        info!("registered address {}", ip);
        interfaces.insert(interface, sock);
    }

    interfaces
}

fn udp_socket(addr: SocketAddr) -> Result<Socket, io::Error> {
    let domain = match addr {
        SocketAddr::V4(_) => socket2::Domain::IPV4,
        SocketAddr::V6(_) => socket2::Domain::IPV6,
    };

    let fd = Socket::new(domain, socket2::Type::DGRAM, None)?;

    fd.set_reuse_address(true)?;

    #[cfg(unix)] // this is currently restricted to Unix's in socket2
    fd.set_reuse_port(true)?;

    fd.set_nonblocking(true)?;

    fd.bind(&addr.into())?;

    Ok(fd)
}
