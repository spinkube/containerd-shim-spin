#[cfg(test)]
mod test {
    use std::{
        process::Command,
        thread,
        time::{self, Duration},
    };

    use anyhow::{Context, Result};
    use curl::easy::Easy;
    use redis::AsyncCommands;

    const RETRY_TIMES: u32 = 5;
    const INTERVAL_IN_SECS: u64 = 10;

    pub async fn retry_get(url: &str, retry_times: u32, interval_in_secs: u64) -> Result<Vec<u8>> {
        let mut i = 0;
        let mut handle = Easy::new();
        handle.url(url)?;
        let mut buf = Vec::new();
        loop {
            let res = {
                let mut transfer = handle.transfer();
                transfer.write_function(|data| {
                    buf.extend_from_slice(data);
                    Ok(data.len())
                })?;
                transfer.perform()
            };
            let response_code = handle.response_code()?;
            // verify res is ok and not 404
            match res {
                Ok(_) => {
                    println!("response_code: {}", response_code);
                    if response_code / 100 == 2 {
                        break; // 2xx response
                    }
                }
                Err(e) => {
                    println!("res: {}, response_code: {}", e, response_code);
                }
            }
            i += 1;
            if i == retry_times {
                anyhow::bail!("failed to curl for {}", url);
            }
            buf = Vec::new();
            tokio::time::sleep(Duration::from_secs(interval_in_secs)).await;
        }
        Ok(buf)
    }

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

        let forward_port = port_forward_svc(redis_port, "redis").await?;

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

        let forward_port = port_forward_svc(redis_port, "redis").await?;

        let client = redis::Client::open(format!("redis://localhost:{}", forward_port))
            .context("connecting to redis")?;
        let mut con = client.get_multiplexed_async_connection().await?;

        con.publish::<_, _, ()>("testchannel", "some-payload")
            .await?;

        let one_sec = time::Duration::from_secs(1);
        thread::sleep(one_sec);

        let value = con.get::<&str, String>("redis-trigger-app-key").await?;
        assert_eq!(value, "redis-trigger-app-value");

        Ok(())
    }

    #[tokio::test]
    async fn spin_mqtt_trigger_app_test() -> Result<()> {
        use std::time::Duration;
        let mqtt_port = 1883;
        let message = "MESSAGE";
        let iterations = 5;

        // Ensure kubectl is in PATH
        if !is_kubectl_installed().await? {
            anyhow::bail!("kubectl is not installed");
        }

        // Port forward the emqx mqtt broker
        let forward_port = port_forward_svc(mqtt_port, "emqx").await?;

        // Publish a message to the emqx broker
        let mut mqttoptions = rumqttc::MqttOptions::new("123", "127.0.0.1", forward_port);
        mqttoptions.set_keep_alive(std::time::Duration::from_secs(1));

        let (client, mut eventloop) = rumqttc::AsyncClient::new(mqttoptions, 10);
        client
            .subscribe(
                "containerd-shim-spin/mqtt-test-17h24d",
                rumqttc::QoS::AtMostOnce,
            )
            .await
            .unwrap();

        // Publish a message several times for redundancy
        tokio::task::spawn(async move {
            for _i in 0..iterations {
                client
                    .publish(
                        "containerd-shim-spin/mqtt-test-17h24d",
                        rumqttc::QoS::AtLeastOnce,
                        false,
                        message.as_bytes(),
                    )
                    .await
                    .unwrap();
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });
        tokio::time::sleep(Duration::from_secs(5)).await;

        // Poll the event loop to ensure messages are published
        for _i in 0..iterations {
            eventloop.poll().await?;
        }
        thread::sleep(time::Duration::from_secs(5));

        // Ensure that the message was received and logged by the spin app
        let log = get_logs_by_label("app=spin-mqtt-message-logger").await?;
        assert!(log.contains(message));
        Ok(())
    }

    #[tokio::test]
    async fn spin_static_assets_test() -> Result<()> {
        let host_port = 8082;

        // curl for static asset
        println!(
            " >>> curl http://localhost:{}/static-assets/jabberwocky.txt",
            host_port
        );
        let res = retry_get(
            &format!(
                "http://localhost:{}/static-assets/jabberwocky.txt",
                host_port
            ),
            RETRY_TIMES,
            INTERVAL_IN_SECS,
        )
        .await?;
        assert!(String::from_utf8_lossy(&res).contains("'Twas brillig, and the slithy toves"));

        Ok(())
    }

    async fn is_kubectl_installed() -> anyhow::Result<bool> {
        let output: Result<std::process::Output, std::io::Error> =
            tokio::process::Command::new("kubectl")
                .arg("version")
                .arg("--client")
                .output()
                .await;

        match output {
            Ok(output) => Ok(output.status.success()),
            Err(_) => Ok(false),
        }
    }

    async fn port_forward_svc(svc_port: u16, svc_name: &str) -> Result<u16> {
        let port = get_random_port()?;

        println!(
            " >>> kubectl portforward svc {} {}:{} ",
            svc_name, port, svc_port
        );

        Command::new("kubectl")
            .arg("port-forward")
            .arg(svc_name)
            .arg(format!("{}:{}", port, svc_port))
            .spawn()?;
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        Ok(port)
    }

    async fn get_logs_by_label(label: &str) -> Result<String> {
        let output = tokio::process::Command::new("kubectl")
            .arg("logs")
            .arg("-l")
            .arg(label)
            .output()
            .await
            .context("failed to get logs")?;
        let log = std::str::from_utf8(&output.stdout)?;
        Ok(log.to_owned())
    }

    /// Uses a track to get a random unused port
    fn get_random_port() -> anyhow::Result<u16> {
        Ok(std::net::TcpListener::bind("localhost:0")?
            .local_addr()?
            .port())
    }
}
