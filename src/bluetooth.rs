use crate::protocol;
use crate::state::SharedState;
use std::collections::HashMap;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use tokio::io::unix::AsyncFd;
use tokio::runtime::Handle;
use tokio::time::{self, Duration};
use zbus::{interface, proxy, zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value}};

pub const AIRPODS_UUID: &str = "74ec2172-0bad-4d01-8f77-997b2be0722a";

pub struct Profile { 
    pub state: SharedState,
    pub rt_handle: Handle,
}

#[interface(name = "org.bluez.Profile1")]
impl Profile {
    async fn new_connection(&self, _device: ObjectPath<'_>, fd: zbus::zvariant::Fd<'_>, _p: HashMap<String, OwnedValue>) {
        let owned = unsafe { OwnedFd::from_raw_fd(libc::dup(fd.as_raw_fd())) };
        {
            let mut s = self.state.lock().unwrap();
            s.connected = true;
            s.print_json();
        }
        let st = self.state.clone();
        self.rt_handle.spawn(async move {
            run_rfcomm_session(owned, st).await;
        });
    }
    async fn release(&self) {}
    async fn request_disconnection(&self, _d: ObjectPath<'_>) {}
}

async fn run_rfcomm_session(fd: OwnedFd, state: SharedState) {
    let raw_fd = fd.as_raw_fd();
    { state.lock().unwrap().session_fd = Some(raw_fd); }
    
    unsafe {
        let flags = libc::fcntl(raw_fd, libc::F_GETFL);
        libc::fcntl(raw_fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }

    let async_fd = match AsyncFd::new(fd) {
        Ok(f) => f,
        Err(_) => return,
    };

    let _ = unsafe { libc::send(raw_fd, protocol::HANDSHAKE.as_ptr() as _, protocol::HANDSHAKE.len(), 0) };

    let s_hb = state.clone();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(3));
        loop {
            interval.tick().await;
            let fd_val = {
                let s = s_hb.lock().unwrap();
                if !s.connected { break; }
                s.session_fd
            };
            if let Some(f) = fd_val {
                let _ = unsafe { libc::send(f, protocol::REQUEST_NOTIF.as_ptr() as _, protocol::REQUEST_NOTIF.len(), 0) };
            }
        }
    });

    let mut buf = [0u8; 1024];
    loop {
        match async_fd.readable().await {
            Ok(mut guard) => {
                let n = unsafe { libc::recv(raw_fd, buf.as_mut_ptr() as _, buf.len(), 0) };
                if n > 0 {
                    let data = &buf[..n as usize];
                    let mut s = state.lock().unwrap();
   
                    if s.debug_mode {
                        eprintln!("[Raw] {:02x?}", data);
                    }
                    
                    if protocol::parse_data(data, &mut *s) {
                        s.print_json();
                    }
                    guard.retain_ready();
                } else if n == 0 {
                    break;
                } else {
                    let err = std::io::Error::last_os_error();
                    if err.kind() == std::io::ErrorKind::WouldBlock {
                        guard.clear_ready();
                        continue;
                    }
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let mut s = state.lock().unwrap();
    s.connected = false; s.session_fd = None; s.print_json();
}

#[proxy(interface = "org.bluez.Device1", default_service = "org.bluez")]
pub trait Device {
    async fn connect_profile(&self, uuid: &str) -> zbus::Result<()>;
    #[zbus(property)] fn alias(&self) -> zbus::Result<String>;
    #[zbus(property)] fn paired(&self) -> zbus::Result<bool>;
    #[zbus(property)] fn connected(&self) -> zbus::Result<bool>;
    #[zbus(property)] fn modalias(&self) -> zbus::Result<String>;
}

#[proxy(interface = "org.bluez.ProfileManager1", default_service = "org.bluez", default_path = "/org/bluez")]
pub trait ProfileManager {
    async fn register_profile(&self, profile: ObjectPath<'_>, uuid: &str, options: HashMap<&str, Value<'_>>) -> zbus::Result<()>;
    async fn unregister_profile(&self, profile: ObjectPath<'_>) -> zbus::Result<()>;
}

#[proxy(interface = "org.freedesktop.DBus.ObjectManager", default_service = "org.bluez", default_path = "/")]
pub trait ObjectManager {
    async fn get_managed_objects(&self) -> zbus::Result<HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedValue>>>>;
}
