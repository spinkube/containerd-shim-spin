use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::Context as _;
use test_environment::{
    http::{Request, Response},
    io::OutputStream,
    services::ServicesConfig,
    Runtime, TestEnvironment, TestEnvironmentConfig,
};

fn main() {
    let tests_dir = conformance_tests::download_tests().unwrap();
    let mut args = std::env::args().skip(1);
    let spin_binary = &args
        .next()
        .expect("expected first arg to be path to Spin binary");
    let ctr_binary = &args
        .next()
        .expect("expected second arg to be path to ctr binary");

    'test: for test in conformance_tests::tests(&tests_dir).unwrap() {
        println!("running test: {}", test.name);
        let mut services = Vec::new();
        for precondition in &test.config.preconditions {
            match precondition {
                conformance_tests::config::Precondition::HttpEcho => {
                    services.push("http-echo".into());
                }
                conformance_tests::config::Precondition::KeyValueStore(k) => {
                    if k.label != "default" {
                        panic!("unsupported label: {}", k.label);
                    }
                }
            }
        }
        // Just using TTL.sh until we decide where to host these (local registry, ghcr, etc)
        let oci_image = format!("ttl.sh/{}:72h", test.name);
        let env_config = SpinShim::config(
            ctr_binary.into(),
            spin_binary.into(),
            oci_image.clone(),
            test_environment::services::ServicesConfig::new(services).unwrap(),
        );
        let mut env = TestEnvironment::up(env_config, move |e| {
            let mut manifest =
                test_environment::manifest_template::EnvTemplate::from_file(&test.manifest)
                    .unwrap();
            manifest.substitute(e, |_| None).unwrap();
            e.write_file("spin.toml", manifest.contents())?;
            e.copy_into(&test.component, test.component.file_name().unwrap())?;
            Ok(())
        })
        .unwrap();
        for invocation in test.config.invocations {
            let conformance_tests::config::Invocation::Http(mut invocation) = invocation;
            invocation.request.substitute_from_env(&mut env).unwrap();
            let shim = env.runtime_mut();
            if let Err(e) = invocation.run(|request| shim.make_http_request(request)) {
                println!("❌ test failed: {}", test.name);
                println!("error: {}", e);
                for e in e.chain() {
                    println!("\t{}", e);
                }

                continue 'test;
            }
        }
        println!("✅ test passed: {}", test.name);
    }
}

struct SpinShim {
    process: std::process::Child,
    #[allow(dead_code)]
    stdout: OutputStream,
    stderr: OutputStream,
    io_mode: IoMode,
}

/// `ctr run` invocations require an ID that is unique to all currently running instances. Since
/// only one test runs at a time, we can reuse a constant ID.
const CTR_RUN_ID: &str = "run-id";

impl SpinShim {
    pub fn config(
        ctr_binary: PathBuf,
        spin_binary: PathBuf,
        oci_image: String,
        services_config: ServicesConfig,
    ) -> TestEnvironmentConfig<SpinShim> {
        TestEnvironmentConfig {
            services_config,
            create_runtime: Box::new(move |env| {
                SpinShim::regisry_push(&spin_binary, &oci_image, env)?;
                SpinShim::image_pull(&ctr_binary, &oci_image)?;
                SpinShim::start(&ctr_binary, env, &oci_image, CTR_RUN_ID)
            }),
        }
    }

    pub fn regisry_push<R>(
        spin_binary_path: &Path,
        image: &str,
        env: &mut TestEnvironment<R>,
    ) -> anyhow::Result<()> {
        // TODO: consider enabling configuring a port
        let mut cmd = Command::new(spin_binary_path);
        cmd.args(["registry", "push"]).arg(image);
        env.run_in(&mut cmd)
            .context("failed to push spin app to registry with 'spin'")?;
        Ok(())
    }

    pub fn image_pull(ctr_binary_path: &Path, image: &str) -> anyhow::Result<()> {
        // TODO: consider enabling configuring a port
        Command::new(ctr_binary_path)
            .args(["image", "pull"])
            .arg(image)
            .output()
            .context("failed to pull spin app with 'ctr'")?;
        // TODO: assess output
        Ok(())
    }

    /// Start the Spin app using `ctr run`
    /// Equivalent of `sudo ctr run --rm --net-host --runtime io.containerd.spin.v2 ttl.sh/myapp:48h ctr-run-id bogus-arg` for image `ttl.sh/myapp:48h` and run id `ctr-run-id`
    pub fn start<R>(
        ctr_binary_path: &Path,
        env: &mut TestEnvironment<R>,
        image: &str,
        ctr_run_id: &str,
    ) -> anyhow::Result<Self> {
        let port = 80;
        let mut ctr_cmd = std::process::Command::new(ctr_binary_path);
        let child = ctr_cmd
            .arg("run")
            .args(["--rm", "--net-host", "--runtime", "io.containerd.spin.v2"])
            .arg(image)
            .arg(ctr_run_id)
            .arg("bogus-arg")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (key, value) in env.env_vars() {
            child.env(key, value);
        }
        let mut child = child.spawn()?;
        let stdout = OutputStream::new(child.stdout.take().unwrap());
        let stderr = OutputStream::new(child.stderr.take().unwrap());
        log::debug!("Awaiting shim binary to start up on port {port}...");
        let mut spin = Self {
            process: child,
            stdout,
            stderr,
            io_mode: IoMode::Http(port),
        };
        let start = std::time::Instant::now();
        loop {
            match std::net::TcpStream::connect(format!("127.0.0.1:{port}")) {
                Ok(_) => {
                    log::debug!("Spin shim started on port {port}.");
                    return Ok(spin);
                }
                Err(e) => {
                    let stderr = spin.stderr.output_as_str().unwrap_or("<non-utf8>");
                    log::trace!("Checking that the shim server started returned an error: {e}");
                    log::trace!("Current spin stderr = '{stderr}'");
                }
            }
            if let Some(status) = spin.try_wait()? {
                anyhow::bail!(
                    "Shim exited early with status code {:?}\n{}{}",
                    status.code(),
                    spin.stdout.output_as_str().unwrap_or("<non-utf8>"),
                    spin.stderr.output_as_str().unwrap_or("<non-utf8>")
                );
            }

            if start.elapsed() > std::time::Duration::from_secs(2 * 60) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        anyhow::bail!(
            "`ctr run` did not start server or error after two minutes. stderr:\n\t{}",
            spin.stderr.output_as_str().unwrap_or("<non-utf8>")
        )
    }

    /// Make an HTTP request against Spin
    ///
    /// Will fail if Spin has already exited or if the io mode is not HTTP
    pub fn make_http_request(&mut self, request: Request<'_, String>) -> anyhow::Result<Response> {
        let IoMode::Http(port) = self.io_mode;
        if let Some(status) = self.try_wait()? {
            anyhow::bail!(
                "make_http_request - shim exited early with status code {:?}",
                status.code()
            );
        }
        log::debug!("Connecting to HTTP server on port {port}...");
        let response = request.send("localhost", port)?;
        log::debug!("Awaiting response from server");
        if let Some(status) = self.try_wait()? {
            anyhow::bail!("Spin exited early with status code {:?}", status.code());
        }
        Ok(response)
    }

    pub fn stderr(&mut self) -> &str {
        self.stderr.output_as_str().unwrap_or("<non-utf8>")
    }

    fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.process.try_wait()
    }
}

impl Drop for SpinShim {
    fn drop(&mut self) {
        kill_process(&mut self.process);
    }
}

impl Runtime for SpinShim {
    fn error(&mut self) -> anyhow::Result<()> {
        if self.try_wait()?.is_some() {
            anyhow::bail!("Containerd shim spin exited early: {}", self.stderr());
        }

        Ok(())
    }
}

/// How this instance is communicating with the outside world
pub enum IoMode {
    /// An http server is running on this port
    Http(u16),
}

/// Helper function to kill a process
fn kill_process(process: &mut std::process::Child) {
    #[cfg(windows)]
    {
        let _ = process.kill();
    }
    #[cfg(not(windows))]
    {
        let pid = nix::unistd::Pid::from_raw(process.id() as i32);
        let _ = nix::sys::signal::kill(pid, nix::sys::signal::SIGTERM);
    }
}
