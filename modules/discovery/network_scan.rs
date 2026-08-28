use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;
use std::thread;
use std::sync::{Arc, Mutex};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct DiscoveredHost {
    pub ip: IpAddr,
    pub mac: Option<String>,
    pub hostname: Option<String>,
    pub open_ports: Vec<u16>,
    pub shares: Vec<String>,
}

pub struct NetworkScanner;

impl NetworkScanner {
    pub fn scan_network() -> Vec<DiscoveredHost> {
        let mut hosts = Vec::new();
        
        // Get local IP range
        let local_ip = Self::get_local_ip();
        let network = Self::get_network_range(local_ip);
        
        println!("Scanning network: {:?}", network);
        
        // ARP scan for live hosts
        let live_hosts = Self::arp_scan(&network);
        
        // Port scan each live host
        for ip in live_hosts {
            let mut host = DiscoveredHost {
                ip: IpAddr::V4(ip),
                mac: Self::get_mac_address(ip),
                hostname: Self::resolve_hostname(ip),
                open_ports: Self::port_scan(ip),
                shares: Vec::new(),
            };
            
            // Check for SMB shares if port 445 is open
            if host.open_ports.contains(&445) {
                host.shares = Self::enumerate_shares(ip);
            }
            
            hosts.push(host);
        }
        
        hosts
    }
    
    fn get_local_ip() -> Ipv4Addr {
        // Get local IP from socket
        if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
            if let Ok(_) = socket.connect("8.8.8.8:80") {
                if let Ok(addr) = socket.local_addr() {
                    if let IpAddr::V4(ip) = addr.ip() {
                        return ip;
                    }
                }
            }
        }
        
        Ipv4Addr::new(192, 168, 1, 1)
    }
    
    fn get_network_range(ip: Ipv4Addr) -> Vec<Ipv4Addr> {
        // Assume /24 network
        let octets = ip.octets();
        let base = Ipv4Addr::new(octets[0], octets[1], octets[2], 1);
        let end = Ipv4Addr::new(octets[0], octets[1], octets[2], 254);
        
        let start_u32 = u32::from(base);
        let end_u32 = u32::from(end);
        
        (start_u32..=end_u32)
            .map(Ipv4Addr::from)
            .collect()
    }
    
    fn arp_scan(network: &[Ipv4Addr]) -> Vec<Ipv4Addr> {
        let live_hosts = Arc::new(Mutex::new(Vec::new()));
        let mut handles = vec![];
        
        // Scan in parallel batches
        for chunk in network.chunks(50) {
            let live = Arc::clone(&live_hosts);
            let ips: Vec<Ipv4Addr> = chunk.to_vec();
            
            let handle = thread::spawn(move || {
                for ip in ips {
                    if Self::is_host_alive(ip) {
                        live.lock().unwrap().push(ip);
                    }
                }
            });
            
            handles.push(handle);
        }
        
        for handle in handles {
            let _ = handle.join();
        }
        
        Arc::try_unwrap(live_hosts).unwrap().into_inner().unwrap()
    }
    
    fn is_host_alive(ip: Ipv4Addr) -> bool {
        // Try ICMP ping first
        let output = Command::new("ping")
            .args(&["-n", "1", "-w", "500", &ip.to_string()])
            .output();
        
        if let Ok(out) = output {
            if out.status.success() {
                return true;
            }
        }
        
        // Fallback: try common ports
        for port in &[135, 139, 445, 22, 80, 443] {
            if Self::tcp_connect(ip, *port) {
                return true;
            }
        }
        
        false
    }
    
    fn tcp_connect(ip: Ipv4Addr, port: u16) -> bool {
        let addr = SocketAddr::new(IpAddr::V4(ip), port);
        TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok()
    }
    
    fn port_scan(ip: Ipv4Addr) -> Vec<u16> {
        let common_ports = vec![
            21, 22, 23, 25, 53, 80, 110, 135, 139, 143, 443, 445,
            993, 995, 1433, 1521, 3306, 3389, 5432, 5900, 8080,
        ];
        
        let open_ports = Arc::new(Mutex::new(Vec::new()));
        let mut handles = vec![];
        
        for port in common_ports {
            let open = Arc::clone(&open_ports);
            let handle = thread::spawn(move || {
                if Self::tcp_connect(ip, port) {
                    open.lock().unwrap().push(port);
                }
            });
            handles.push(handle);
        }
        
        for handle in handles {
            let _ = handle.join();
        }
        
        Arc::try_unwrap(open_ports).unwrap().into_inner().unwrap()
    }
    
    fn enumerate_shares(ip: Ipv4Addr) -> Vec<String> {
        let mut shares = Vec::new();
        
        let output = Command::new("net")
            .args(&["view", &format!("\\\\{}", ip), "/all"])
            .output();
        
        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                if line.contains("Disk") && !line.contains("command") {
                    let parts: Vec<&str> = line.trim().split_whitespace().collect();
                    if !parts.is_empty() {
                        shares.push(parts[0].to_string());
                    }
                }
            }
        }
        
        shares
    }
    
    fn resolve_hostname(ip: Ipv4Addr) -> Option<String> {
        let output = Command::new("nslookup")
            .args(&[&ip.to_string()])
            .output()
            .ok()?;
        
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            if line.contains("Name:") {
                return line.splitn(2, "Name:").nth(1).map(|s| s.trim().to_string());
            }
        }
        
        None
    }
    
    fn get_mac_address(ip: Ipv4Addr) -> Option<String> {
        let output = Command::new("arp")
            .args(&["-a", &ip.to_string()])
            .output()
            .ok()?;
        
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            if line.contains(&ip.to_string()) {
                // Parse MAC from ARP output
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    return Some(parts[1].to_string());
                }
            }
        }
        
        None
    }
}