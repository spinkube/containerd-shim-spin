use anyhow::{Result, anyhow};
use bytes::Bytes;
use spin_sdk::redis_component;
use std::str::from_utf8;
use spin_sdk::{redis, variables};

/// A simple Spin Redis component.
#[redis_component]
fn on_message(message: Bytes) -> Result<()> {
    let conn = redis::Connection::open("redis://redis-service.default.svc.cluster.local:6379")?;
    println!("{}", from_utf8(&message)?);

    conn.set("spin-multi-trigger-app-key", &"spin-multi-trigger-app-value".to_owned().into_bytes())
    .map_err(|_| anyhow!("Error executing Redis set command"))?;

    Ok(())
}
