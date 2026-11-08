//! Copied unchanged from bacnetio (concept section 14, answer 6).
//!
//! Network adapters: list them, pick the one that most likely leads to the building network, and
//! derive the broadcast address and the addresses to ask one by one.

use std::net::Ipv4Addr;

use anyhow::{Context, bail};

/// Name parts of adapters that belong to VPNs, virtual machines or containers, never to a plant
/// network.
const VIRTUAL_NAME_HINTS: &[&str] = &[
    "vpn",
    "virtual",
    "vethernet",
    "hyper-v",
    "docker",
    "wsl",
    "vmware",
    "virtualbox",
    "tailscale",
    "zerotier",
    "wireguard",
    "tun",
    "tap",
    "br-",
    "veth",
];

/// Name parts of wireless adapters. On site the plant network is usually the cable, while Wi-Fi
/// leads to the office or a hotspot.
const WIRELESS_NAME_HINTS: &[&str] = &["wlan", "wi-fi", "wifi", "wireless", "wlp", "wlo"];

/// Name parts of mobile data: Android's `ccmni`/`rmnet`, `wwan` on Linux, Windows' "Cellular" or
/// "Mobile Broadband". No plant is reached that way, so it never comes before Wi-Fi, a cable or a
/// VPN.
const CELLULAR_NAME_HINTS: &[&str] = &["ccmni", "rmnet", "wwan", "cellular", "mobile broadband"];

/// Smallest prefix length (largest subnet) still asked address by address: /22 has 1022 hosts.
pub const MIN_SWEEP_PREFIX_LEN: u8 = 22;

/// What an adapter most likely is, for picking the right one at a glance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AdapterKind {
    Cable,
    Wireless,
    Virtual,
}

/// An IPv4 subnet, such as the plant's network behind a VPN.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Subnet {
    pub address: Ipv4Addr,
    pub prefix_len: u8,
}

impl Subnet {
    /// "192.168.10.0/24"
    pub fn parse(text: &str) -> Result<Subnet, String> {
        let invalid = || format!("{text} is not a subnet like 192.168.10.0/24");
        let (address, prefix_len) = text.trim().split_once('/').ok_or_else(invalid)?;
        let address = address.parse().map_err(|_| invalid())?;
        let prefix_len = prefix_len.parse().map_err(|_| invalid())?;
        if prefix_len > 32 {
            return Err(invalid());
        }
        Ok(Subnet {
            address,
            prefix_len,
        })
    }

    fn netmask(&self) -> u32 {
        u32::MAX
            .checked_shl(32 - u32::from(self.prefix_len))
            .unwrap_or(0)
    }

    pub fn broadcast(&self) -> Ipv4Addr {
        Ipv4Addr::from(u32::from(self.address) | !self.netmask())
    }

    /// Every host address except `own`, or `None` if the subnet is too large to ask one by one.
    pub fn hosts(&self, own: Option<Ipv4Addr>) -> Option<Vec<Ipv4Addr>> {
        if self.prefix_len < MIN_SWEEP_PREFIX_LEN {
            return None;
        }
        let network = u32::from(self.address) & self.netmask();
        let broadcast = u32::from(self.broadcast());
        // /31 and /32 have no network and broadcast address to leave out
        let (first, last) = if self.prefix_len >= 31 {
            (network, broadcast)
        } else {
            (network + 1, broadcast - 1)
        };
        Some(
            (first..=last)
                .map(Ipv4Addr::from)
                .filter(|&address| Some(address) != own)
                .collect(),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Adapter {
    pub name: String,
    pub ip: Ipv4Addr,
    pub prefix_len: u8,
    pub point_to_point: bool,
    /// What the adapter is, where the system says it apart from the name: "VirtualBox
    /// Host-Only Ethernet Adapter" behind "Ethernet 2" on Windows.
    pub description: Option<String>,
}

/// All IPv4 adapters that are up; loopback only for the browser tests, whose simulated devices
/// live there.
pub fn list(loopback: bool) -> std::io::Result<Vec<Adapter>> {
    let descriptions = descriptions()?;
    let adapters = if_addrs::get_if_addrs()?
        .into_iter()
        .filter(|interface| interface.is_oper_up() && (loopback || !interface.is_loopback()))
        .filter_map(|interface| match interface.addr {
            if_addrs::IfAddr::V4(v4) => Some(Adapter {
                ip: v4.ip,
                prefix_len: v4.prefixlen,
                point_to_point: interface.is_p2p,
                description: descriptions.get(&interface.name).cloned(),
                name: interface.name,
            }),
            if_addrs::IfAddr::V6(_) => None,
        })
        .collect();
    Ok(adapters)
}

#[cfg(windows)]
fn descriptions() -> std::io::Result<std::collections::HashMap<String, String>> {
    crate::adapter_descriptions::descriptions()
}

/// Other systems name adapters by their driver ("wlp0s20f3", "tailscale0"); the name says enough.
#[cfg(not(windows))]
fn descriptions() -> std::io::Result<std::collections::HashMap<String, String>> {
    Ok(std::collections::HashMap::new())
}

/// The adapter with the given address, or the default one.
pub fn choose(interface: Option<Ipv4Addr>, loopback: bool) -> anyhow::Result<Adapter> {
    let adapters = list(loopback).context("cannot list network adapters")?;
    let chosen = match interface {
        Some(ip) => adapters.iter().find(|adapter| adapter.ip == ip),
        None => pick_default(&adapters),
    };
    match (chosen, interface) {
        (Some(adapter), _) => Ok(adapter.clone()),
        (None, Some(ip)) => bail!("no network adapter has the address {ip}"),
        (None, None) => bail!("no network adapter with an IPv4 address is connected"),
    }
}

/// The adapter that most likely leads to the building network; the first one on a tie.
pub fn pick_default(adapters: &[Adapter]) -> Option<&Adapter> {
    let mut best: Option<&Adapter> = None;
    for adapter in adapters {
        if best.is_none_or(|current| adapter.score() > current.score()) {
            best = Some(adapter);
        }
    }
    best
}

impl Adapter {
    /// The adapter's own subnet.
    pub fn subnet(&self) -> Subnet {
        Subnet {
            address: self.ip,
            prefix_len: self.prefix_len,
        }
    }

    pub fn broadcast(&self) -> Ipv4Addr {
        self.subnet().broadcast()
    }

    /// Every other address in the subnet, or `None` if the subnet is too large to ask one by one.
    pub fn sweep_targets(&self) -> Option<Vec<Ipv4Addr>> {
        self.subnet().hosts(Some(self.ip))
    }

    pub fn kind(&self) -> AdapterKind {
        if self.point_to_point || self.name_contains_any(VIRTUAL_NAME_HINTS) {
            AdapterKind::Virtual
        } else if self.name_contains_any(WIRELESS_NAME_HINTS)
            || self.name_contains_any(CELLULAR_NAME_HINTS)
        {
            AdapterKind::Wireless
        } else {
            AdapterKind::Cable
        }
    }

    fn name_contains_any(&self, hints: &[&str]) -> bool {
        let text = format!(
            "{} {}",
            self.name,
            self.description.as_deref().unwrap_or_default()
        )
        .to_lowercase();
        hints.iter().any(|hint| text.contains(hint))
    }

    fn score(&self) -> u8 {
        let mut score = 0;
        if !self.ip.is_loopback() {
            score += 16;
        }
        if !self.name_contains_any(VIRTUAL_NAME_HINTS)
            && !self.name_contains_any(CELLULAR_NAME_HINTS)
        {
            score += 8;
        }
        if !self.name_contains_any(WIRELESS_NAME_HINTS)
            && !self.name_contains_any(CELLULAR_NAME_HINTS)
        {
            score += 4;
        }
        if !self.point_to_point {
            score += 2;
        }
        if self.ip.is_private() {
            score += 2;
        }
        if !self.ip.is_link_local() {
            score += 1;
        }
        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adapter(name: &str, ip: [u8; 4], prefix_len: u8) -> Adapter {
        Adapter {
            name: name.into(),
            ip: Ipv4Addr::from(ip),
            prefix_len,
            point_to_point: false,
            description: None,
        }
    }

    #[test]
    fn broadcast_of_common_subnets() {
        assert_eq!(
            adapter("eth0", [192, 168, 10, 23], 24).broadcast(),
            Ipv4Addr::new(192, 168, 10, 255)
        );
        assert_eq!(
            adapter("eth0", [10, 1, 2, 3], 16).broadcast(),
            Ipv4Addr::new(10, 1, 255, 255)
        );
        assert_eq!(
            adapter("eth0", [10, 0, 0, 1], 0).broadcast(),
            Ipv4Addr::BROADCAST
        );
    }

    #[test]
    fn sweep_leaves_out_network_broadcast_and_own_address() {
        let targets = adapter("eth0", [192, 168, 10, 23], 24)
            .sweep_targets()
            .unwrap();
        assert_eq!(targets.len(), 253);
        assert_eq!(targets.first(), Some(&Ipv4Addr::new(192, 168, 10, 1)));
        assert_eq!(targets.last(), Some(&Ipv4Addr::new(192, 168, 10, 254)));
        assert!(!targets.contains(&Ipv4Addr::new(192, 168, 10, 23)));
    }

    #[test]
    fn sweep_of_point_to_point_subnet() {
        let targets = adapter("eth0", [10, 0, 0, 0], 31).sweep_targets().unwrap();
        assert_eq!(targets, vec![Ipv4Addr::new(10, 0, 0, 1)]);
    }

    #[test]
    fn no_sweep_for_large_subnets() {
        assert!(adapter("eth0", [10, 1, 2, 3], 16).sweep_targets().is_none());
        let plant = Subnet::parse("192.168.10.0/24").unwrap();
        let hosts = plant.hosts(None).unwrap();
        assert_eq!(hosts.len(), 254);
        assert_eq!(hosts[0], Ipv4Addr::new(192, 168, 10, 1));
        assert!(Subnet::parse("192.168.10.0").is_err());
        assert!(Subnet::parse("192.168.10.0/33").is_err());
        assert_eq!(
            adapter("eth0", [10, 1, 2, 3], MIN_SWEEP_PREFIX_LEN)
                .sweep_targets()
                .map(|targets| targets.len()),
            Some(1021)
        );
    }

    #[test]
    fn default_prefers_plain_ethernet_over_vpn_and_virtual_adapters() {
        let mut vpn = adapter("Corporate VPN", [10, 8, 0, 5], 24);
        vpn.point_to_point = true;
        let adapters = vec![
            vpn,
            adapter("vEthernet (WSL)", [172, 20, 0, 1], 20),
            adapter("WLAN", [192, 168, 1, 40], 24),
            adapter("Ethernet", [192, 168, 10, 23], 24),
        ];
        assert_eq!(pick_default(&adapters).unwrap().name, "Ethernet");
    }

    #[test]
    fn default_prefers_wifi_over_mobile_data() {
        // a phone: mobile data has a public-looking address and no wireless name
        let adapters = vec![
            adapter("ccmni1", [100, 95, 203, 218], 8),
            adapter("wlan0", [192, 168, 178, 105], 24),
        ];
        assert_eq!(pick_default(&adapters).unwrap().name, "wlan0");
        assert_eq!(adapters[0].kind(), AdapterKind::Wireless);
    }

    #[test]
    fn default_prefers_vpn_over_mobile_data() {
        // a phone away from the plant, reaching it through a VPN app
        let mut vpn = adapter("tun0", [192, 168, 10, 222], 24);
        vpn.point_to_point = true;
        let adapters = vec![adapter("ccmni2", [100, 81, 242, 155], 8), vpn];
        assert_eq!(pick_default(&adapters).unwrap().name, "tun0");
    }

    #[test]
    fn default_avoids_link_local() {
        let adapters = vec![
            adapter("Ethernet", [169, 254, 3, 4], 16),
            adapter("Ethernet 2", [192, 168, 10, 23], 24),
        ];
        assert_eq!(pick_default(&adapters).unwrap().name, "Ethernet 2");
    }

    #[test]
    fn kind_from_name_and_link() {
        assert_eq!(
            adapter("Ethernet 2", [10, 0, 0, 2], 24).kind(),
            AdapterKind::Cable
        );
        assert_eq!(
            adapter("wlp0s20f3", [10, 0, 0, 2], 24).kind(),
            AdapterKind::Wireless
        );
        assert_eq!(
            adapter("tailscale0", [100, 1, 2, 3], 32).kind(),
            AdapterKind::Virtual
        );
        let mut vpn = adapter("Office", [10, 8, 0, 5], 24);
        vpn.point_to_point = true;
        assert_eq!(vpn.kind(), AdapterKind::Virtual);
    }

    #[test]
    fn windows_descriptions_reveal_virtual_and_wireless_adapters() {
        let mut host_only = adapter("Ethernet 2", [192, 168, 56, 1], 24);
        host_only.description = Some("VirtualBox Host-Only Ethernet Adapter".into());
        let mut wireless = adapter("WLAN 3", [192, 168, 1, 40], 24);
        wireless.description = Some("Intel(R) Wi-Fi 6 AX201 160MHz".into());
        assert_eq!(host_only.kind(), AdapterKind::Virtual);
        assert_eq!(wireless.kind(), AdapterKind::Wireless);
        let adapters = vec![host_only, wireless];
        assert_eq!(pick_default(&adapters).unwrap().name, "WLAN 3");
    }

    #[test]
    fn no_default_without_adapters() {
        assert!(pick_default(&[]).is_none());
    }
}
