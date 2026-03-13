mod state;
mod protocol;
mod bluetooth;
mod socket;

use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use zbus::connection::Builder;
use zbus::zvariant::{ObjectPath, Value};
use crate::state::AirPodsState;
use crate::bluetooth::*;

const PROFILE_DBUS_PATH: &str = "/com/airpods/profile";

async fn find_airpods_path(conn: &zbus::Connection) -> Result<String> {
    let om = ObjectManagerProxy::new(conn).await?;
    let objects = om.get_managed_objects().await?;
    for (path, interfaces) in objects {
        if interfaces.contains_key("org.bluez.Device1") {
            let dev = DeviceProxy::builder(conn).path(path.clone())?.build().await?;
            if dev.paired().await? && dev.alias().await?.to_lowercase().contains("airpods") {
                return Ok(path.to_string());
            }
        }
    }
    Err(anyhow!("No paired AirPods found automatically"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let conn = Builder::system()?.build().await?;
    
    let dev_path = if let Some(arg) = std::env::args().nth(1) {
        format!("/org/bluez/hci0/dev_{}", arg.trim().replace(':', "_").to_uppercase())
    } else {
        find_airpods_path(&conn).await?
    };

    let state = Arc::new(Mutex::new(AirPodsState::default()));
    let s_socket = state.clone();
    tokio::spawn(async move { socket::start_listener(s_socket).await });

    conn.object_server()
        .at(PROFILE_DBUS_PATH, Profile { state: state.clone() })
        .await?;

    let dev = DeviceProxy::builder(&conn).path(ObjectPath::try_from(dev_path.clone())?)?.build().await?;
    state.lock().unwrap().device_name = dev.alias().await?;

    let pm = ProfileManagerProxy::new(&conn).await?;
    let mut opts = HashMap::new();
    opts.insert("Role", Value::from("client"));
    opts.insert("PSM", Value::from(25u16));

    let _ = pm.unregister_profile(ObjectPath::try_from(PROFILE_DBUS_PATH)?).await;
    pm.register_profile(ObjectPath::try_from(PROFILE_DBUS_PATH)?, AIRPODS_UUID, opts).await?;
    
    let _ = dev.connect_profile(AIRPODS_UUID).await;

    tokio::signal::ctrl_c().await?;
    let _ = std::fs::remove_file(socket::SOCKET_PATH);
    Ok(())
}
