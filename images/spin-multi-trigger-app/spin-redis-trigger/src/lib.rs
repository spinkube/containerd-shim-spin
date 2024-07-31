use std::str::from_utf8;

use anyhow::{anyhow, Result};
use bytes::Bytes;
use spin_sdk::{redis, redis_component, variables};

#[redis_component]
fn on_message(message: Bytes) -> Result<()> {
    let address = variables::get("redis_address").expect("could not get variable");
    let conn = redis::Connection::open(&address)?;

    println!("{}", from_utf8(&message)?);

    conn.set(
        "redis-trigger-app-key",
        &"redis-trigger-app-value".to_owned().into_bytes(),
    )
    .map_err(|_| anyhow!("Error executing Redis set command"))?;

    Ok(())
}
