use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::time::Duration;

use windows::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_SUCCESS};
use windows::Win32::NetworkManagement::IpHelper::{
    GAA_FLAG_INCLUDE_GATEWAYS, GAA_FLAG_INCLUDE_PREFIX, GetAdaptersAddresses,
    IP_ADAPTER_ADDRESSES_LH,
};
use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
use windows::Win32::Networking::WinSock::{AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR_IN6};

use crate::Config;

impl Config {
    // https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getadaptersaddresses
    pub fn load() -> std::io::Result<Self> {
        let servers = adapter_addresses()?;

        Ok(Config {
            ndots: 1,
            timeout: Duration::from_secs(5),
            attempts: 2,
            servers,
            ..Default::default()
        })
    }
}

// adapter_addresses returns a list of IP address
fn adapter_addresses() -> std::io::Result<Vec<SocketAddr>> {
    let mut len: u32 = 16 * 1024;
    let mut buf = vec![0u8; len as usize];
    loop {
        buf.reserve(len as usize);

        let ret = unsafe {
            GetAdaptersAddresses(
                AF_UNSPEC.0 as u32,
                GAA_FLAG_INCLUDE_PREFIX | GAA_FLAG_INCLUDE_GATEWAYS,
                None,
                Some(buf.as_mut_ptr() as *mut _),
                &mut len,
            )
        };

        if ret == ERROR_SUCCESS.0 {
            if len == 0 {
                return Ok(vec![]);
            }

            break;
        }

        if ret != ERROR_BUFFER_OVERFLOW.0 {
            return Err(std::io::Error::other(format!(
                "GetAdaptersAddresses failed with error: {}",
                ret
            )));
        }

        if len <= buf.len() as u32 {
            return Err(std::io::Error::other(format!(
                "GetAdaptersAddresses failed with error: {}",
                ret
            )));
        }
    }

    let mut servers = vec![];
    let mut current: *mut IP_ADAPTER_ADDRESSES_LH = buf.as_mut_ptr() as *mut _;
    while !current.is_null() {
        let iaa = unsafe { &*current };
        current = iaa.Next;

        // Only take interfaces whose OperStatus is IfOperStatusUp(0x01) into DNS config
        if iaa.OperStatus != IfOperStatusUp {
            continue;
        }

        // Only take interfaces which have at least one gateway
        if iaa.FirstGatewayAddress.is_null() {
            continue;
        }

        let mut dns = iaa.FirstDnsServerAddress;
        while !dns.is_null() {
            let ptr = unsafe { &*dns };
            dns = ptr.Next;

            let raw_addr = unsafe { &*(ptr.Address.lpSockaddr) };
            let addr = match raw_addr.sa_family {
                AF_INET => {
                    let ip = Ipv4Addr::new(
                        raw_addr.sa_data[0] as u8,
                        raw_addr.sa_data[1] as u8,
                        raw_addr.sa_data[2] as u8,
                        raw_addr.sa_data[3] as u8,
                    );

                    SocketAddr::V4(SocketAddrV4::new(ip, 53))
                }
                AF_INET6 => {
                    let addr = unsafe { *ptr.Address.lpSockaddr.cast::<SOCKADDR_IN6>() };
                    let octets = unsafe { addr.sin6_addr.u.Byte };
                    if octets[0] == 0xfe && octets[1] == 0xc0 {
                        // fec0/10 IPv6 addresses are site local anycast DNS addresses
                        // Microsoft sets by default if no other IPv6 DNS address is
                        // set. Site local anycast is deprecated since 2004.
                        // see https://datatracker.ietf.org/doc/html/rfc3879
                        continue;
                    }

                    let ip = Ipv6Addr::from(unsafe { addr.sin6_addr.u.Byte });
                    SocketAddr::V6(SocketAddrV6::new(ip, 53, 0, 0))
                }
                _ => {
                    // EAFNOSUPPORT
                    continue;
                }
            };

            servers.push(addr);
        }
    }

    Ok(servers)
}
