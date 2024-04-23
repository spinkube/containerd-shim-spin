#[cfg(test)]
mod test {
    use std::{thread, time};

    use anyhow::{Context, Result};
    use redis::AsyncCommands;
    use tokio::process::Command;

    use crate::retry_get;

    const RETRY_TIMES: u32 = 5;
    const INTERVAL_IN_SECS: u64 = 10;

    #[tokio::test]
    async fn spin_test() -> Result<()> {
        let host_port = 8082;

        // curl for hello
        println!(" >>> curl http://localhost:{}/spin/hello", host_port);
        let res = retry_get(
            &format!("http://localhost:{}/spin/hello", host_port),
            RETRY_TIMES,
            INTERVAL_IN_SECS,
        )
        .await?;
        assert_eq!(String::from_utf8_lossy(&res), "Hello world from Spin!");

        Ok(())
    }

    #[tokio::test]
    async fn spin_keyvalue_test() -> Result<()> {
        let host_port = 8082;

        // curl for hello
        println!(" >>> curl http://localhost:{}/keyvalue/keyvalue", host_port);
        let res = retry_get(
            &format!("http://localhost:{}/keyvalue/keyvalue", host_port),
            RETRY_TIMES,
            INTERVAL_IN_SECS,
        )
        .await?;
        assert_eq!(String::from_utf8_lossy(&res), "wow");

        Ok(())
    }

    #[tokio::test]
    async fn spin_inbound_redis_outbound_redis_test() -> Result<()> {
        let host_port = 8082;
        let redis_port = 6379;

        // Ensure kubectl is in PATH
        if !is_kubectl_installed().await? {
            anyhow::bail!("kubectl is not installed");
        }

        let forward_port = port_forward_redis(redis_port).await?;

        let client = redis::Client::open(format!("redis://localhost:{}", forward_port))?;
        let mut con = client.get_multiplexed_async_connection().await?;

        // curl for hello
        println!(
            " >>> curl http://localhost:{}/outboundredis/hello",
            host_port
        );
        let _ = retry_get(
            &format!("http://localhost:{}/outboundredis/hello", host_port),
            RETRY_TIMES,
            INTERVAL_IN_SECS,
        )
        .await?;

        // Retrieve the value for the key 'spin-example'
        let key: String = con.get("spin-example").await?;
        assert_eq!(key, "Eureka!");

        let key: String = con.get("int-key").await?;
        assert_eq!(key, "1");

        Ok(())
    }

    #[tokio::test]
    async fn spin_multi_trigger_app_test() -> Result<()> {
        let host_port = 8082;

        // curl for hello
        println!(" >>> curl http://localhost:{}/multi-trigger-app", host_port);
        let res = retry_get(
            &format!("http://localhost:{}/multi-trigger-app", host_port),
            RETRY_TIMES,
            INTERVAL_IN_SECS,
        )
        .await?;
        assert_eq!(
            String::from_utf8_lossy(&res),
            "Hello world from multi trigger Spin!"
        );

        let redis_port = 6379;

        // Ensure kubectl is in PATH
        if !is_kubectl_installed().await? {
            anyhow::bail!("kubectl is not installed");
        }

        let forward_port = port_forward_redis(redis_port).await?;

        let client = redis::Client::open(format!("redis://localhost:{}", forward_port))
            .context("connecting to redis")?;
        let mut con = client.get_multiplexed_async_connection().await?;

        con.publish("testchannel", "some-payload").await?;

        let one_sec = time::Duration::from_secs(1);
        thread::sleep(one_sec);

        let exists: bool = con.exists("spin-multi-trigger-app-key").await?;
        assert!(exists, "key 'spin-multi-trigger-app-key' does not exist");

        let value = con
            .get::<&str, String>("spin-multi-trigger-app-key")
            .await?;
        assert_eq!(value, "spin-multi-trigger-app-value");

        Ok(())
    }

    async fn is_kubectl_installed() -> anyhow::Result<bool> {
        let output: Result<std::process::Output, std::io::Error> = Command::new("kubectl")
            .arg("version")
            .arg("--client")
            .output()
            .await;

        match output {
            Ok(output) => Ok(output.status.success()),
            Err(_) => Ok(false),
        }
    }

    async fn port_forward_redis(redis_port: u16) -> Result<u16> {
        let port = get_random_port()?;

        println!(" >>> kubectl portforward redis {}:{} ", port, redis_port);

        Command::new("kubectl")
            .arg("port-forward")
            .arg("redis")
            .arg(format!("{}:{}", port, redis_port))
            .spawn()?;
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        Ok(port)
    }

    /// Uses a track to get a random unused port
    fn get_random_port() -> anyhow::Result<u16> {
        Ok(std::net::TcpListener::bind("localhost:0")?
            .local_addr()?
            .port())
    }
}
