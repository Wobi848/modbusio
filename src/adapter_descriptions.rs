//! Copied unchanged from bacnetio (concept section 14, answer 6).
//!
//! Adapter descriptions on Windows. Windows names adapters "Ethernet 2" or "WLAN"; only the
//! description says "VirtualBox Host-Only Ethernet Adapter" or "Intel(R) Wi-Fi 6 AX201".

use std::collections::HashMap;
use std::io;
use std::ptr;

use windows_sys::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_SUCCESS};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST,
    GAA_FLAG_SKIP_UNICAST, GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH,
};
use windows_sys::Win32::Networking::WinSock::AF_INET;

/// First guess of the buffer size; Windows says how much it needs if this is too small.
const INITIAL_BUFFER_BYTES: u32 = 16 * 1024;

/// The description of every IPv4 adapter, by its name.
pub fn descriptions() -> io::Result<HashMap<String, String>> {
    let flags = GAA_FLAG_SKIP_UNICAST
        | GAA_FLAG_SKIP_ANYCAST
        | GAA_FLAG_SKIP_MULTICAST
        | GAA_FLAG_SKIP_DNS_SERVER;
    let mut size = INITIAL_BUFFER_BYTES;
    loop {
        // u64 elements keep the buffer aligned for the adapter structs
        let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
        let first = buffer.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
        // SAFETY: the buffer is writable for `size` bytes and suitably aligned.
        let code = unsafe {
            GetAdaptersAddresses(u32::from(AF_INET), flags, ptr::null(), first, &mut size)
        };
        match code {
            ERROR_SUCCESS => return Ok(collect(first)),
            ERROR_BUFFER_OVERFLOW => continue,
            _ => return Err(io::Error::from_raw_os_error(code as i32)),
        }
    }
}

fn collect(mut adapter: *const IP_ADAPTER_ADDRESSES_LH) -> HashMap<String, String> {
    let mut found = HashMap::new();
    while !adapter.is_null() {
        // SAFETY: Windows filled a linked list of valid entries, ending with a null pointer.
        let entry = unsafe { &*adapter };
        found.insert(wide_text(entry.FriendlyName), wide_text(entry.Description));
        adapter = entry.Next;
    }
    found
}

fn wide_text(text: *const u16) -> String {
    if text.is_null() {
        return String::new();
    }
    // SAFETY: Windows strings in the adapter list end with a zero.
    unsafe {
        let len = (0..).take_while(|&index| *text.add(index) != 0).count();
        String::from_utf16_lossy(std::slice::from_raw_parts(text, len))
    }
}
