//! Optional UDP room directory and encrypted-payload relay. No game window or microphone.
#[path = "../net.rs"]
#[allow(dead_code)]
mod net;
fn main() -> std::io::Result<()> {
    let address = std::env::args()
        .nth(1)
        .unwrap_or_else(|| format!("0.0.0.0:{}", net::HUB_PORT));
    if address == "--help" {
        println!("viper-server [bind-address:port]\nDefault: 0.0.0.0:42861 UDP. Public hosts register their rooms; private invitations relay by room ID.");
        return Ok(());
    }
    let mut directory = net::Directory::bind(net::resolve(&address)?)?;
    println!(
        "Viper Run directory listening on {}",
        directory.local_addr()?
    );
    loop {
        directory.poll()?;
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}
