mod bluetooth;
mod protocol;
mod socket;
mod state;

use crate::bluetooth::*;
use crate::state::AirPodsState;
use anyhow::{Result, anyhow};
use clap::Parser;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::time::{self, Duration};
use zbus::connection::Builder;
use zbus::zvariant::{ObjectPath, Value};

const PROFILE_DBUS_PATH: &str = "/com/airpods/profile";

fn identify_model(alias: &str, modalias: &str) -> String {
    if modalias.contains("v004Cp") {
        if modalias.contains("p200E") {
            return "AirPods Pro (Gen 1)".into();
        }
        if modalias.contains("p2014") {
            return "AirPods Pro (Gen 2)".into();
        }
        if modalias.contains("p200F") {
            return "AirPods Max".into();
        }
        if modalias.contains("p2013") {
            return "AirPods (Gen 3)".into();
        }
    }
    alias.into()
}

async fn find_airpods(conn: &zbus::Connection) -> Result<(String, String, String)> {
    let om = ObjectManagerProxy::new(conn).await?;
    let objects = om.get_managed_objects().await?;
    for (path, interfaces) in objects {
        if interfaces.contains_key("org.bluez.Device1") {
            let dev = DeviceProxy::builder(conn)
                .path(path.clone())?
                .build()
                .await?;
            let mut alias = dev.alias().await.unwrap_or_default();
            if alias == "AirPods" && dev.connected().await.unwrap_or(false) {
                time::sleep(Duration::from_millis(500)).await;
                alias = dev.alias().await.unwrap_or(alias);
            }
            if alias.to_lowercase().contains("airpods") {
                let mod_str = dev.modalias().await.unwrap_or_default();
                let model = identify_model(&alias, &mod_str);
                return Ok((path.to_string(), alias, model));
            }
        }
    }
    Err(anyhow!("No AirPods found"))
}

#[derive(Parser, Debug)]
#[command(author, version, about = "AirPods Bridge for Linux")]
struct Args {
    #[arg(short, long)]
    debug: bool,

    device_mac: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let conn = Builder::system()?.build().await?;
    let rt_handle = tokio::runtime::Handle::current();

    let state = Arc::new(Mutex::new(AirPodsState {
        debug_mode: args.debug,
        ..AirPodsState::default()
    }));
    let s_socket = state.clone();
    tokio::spawn(async move { socket::start_listener(s_socket).await });

    conn.object_server()
        .at(
            PROFILE_DBUS_PATH,
            Profile {
                state: state.clone(),
                rt_handle,
            },
        )
        .await?;

    let pm = ProfileManagerProxy::new(&conn).await?;
    let mut opts = HashMap::new();
    opts.insert("Role", Value::from("client"));
    opts.insert("PSM", Value::from(25u16));

    let _ = pm
        .unregister_profile(ObjectPath::try_from(PROFILE_DBUS_PATH)?)
        .await;
    pm.register_profile(ObjectPath::try_from(PROFILE_DBUS_PATH)?, AIRPODS_UUID, opts)
        .await?;

    loop {
        let is_connected = { state.lock().unwrap().connected };
        if !is_connected {
            if let Ok((path, alias, model)) = find_airpods(&conn).await {
                let _path_clone = path.clone();
                if let Ok(dev) = DeviceProxy::builder(&conn)
                    .path(ObjectPath::try_from(path)?)?
                    .build()
                    .await
                {
                    {
                        let mut s = state.lock().unwrap();
                        s.device_name = alias;
                        s.model_name = model;
                    }
                    if dev.connected().await.unwrap_or(false) {
                        let _ = dev.connect_profile(AIRPODS_UUID).await;
                    }
                }
            }
        }
        time::sleep(Duration::from_secs(5)).await;
    }
}
