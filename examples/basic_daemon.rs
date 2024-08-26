fn main() {
    tracing_subscriber::fmt().pretty().init();
    mdns_rs::PeerDaemon::new();
}
