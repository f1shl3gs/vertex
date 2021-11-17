use std::collections::HashMap;
use std::future::Future;
use sqlx::mysql::{MySqlConnectOptions, MySqlSslMode};
use sqlx::MySqlPool;
use testcontainers::{Container, Docker, Image, WaitForMessage};


const DEFAULT_TAG: &str = "5.7.36";

#[derive(Debug, Default, Clone)]
pub struct MysqlArgs;

impl IntoIterator for MysqlArgs {
    type Item = String;
    type IntoIter = ::std::vec::IntoIter<String>;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        vec![].into_iter()
    }
}

pub struct Mysql {
    tag: String,
    arguments: MysqlArgs,
    envs: HashMap<String, String>,
}

impl Default for Mysql {
    fn default() -> Self {
        Self {
            tag: DEFAULT_TAG.to_string(),
            envs: HashMap::new(),
            arguments: MysqlArgs,
        }
    }
}

impl Mysql {
    pub fn with_env(mut self, key: &str, value: &str) -> Self {
        self.envs.insert(key.to_string(), value.to_string());
        self
    }

    pub fn with_tag(mut self, tag: impl ToString) -> Self {
        self.tag = tag.to_string();
        self
    }
}

impl Image for Mysql {
    type Args = MysqlArgs;
    type EnvVars = HashMap<String, String>;
    type Volumes = HashMap<String, String>;
    type EntryPoint = std::convert::Infallible;

    fn descriptor(&self) -> String {
        format!("{}:{}", "mysql", self.tag)
    }

    fn wait_until_ready<D: Docker>(&self, container: &Container<'_, D, Self>) {
        // TODO: It's just a workaround, without this sleep() the test will running forever.
        std::thread::sleep(std::time::Duration::from_secs(10));

        container.logs()
            .stderr
            .wait_for_message(r#"mysqld: ready for connections."#)
            .unwrap();
    }

    fn args(&self) -> Self::Args {
        MysqlArgs {}
    }

    fn env_vars(&self) -> Self::EnvVars {
        self.envs.clone()
    }

    fn volumes(&self) -> Self::Volumes {
        HashMap::new()
    }

    fn with_args(self, arguments: Self::Args) -> Self {
        Self {
            arguments,
            ..self
        }
    }
}

pub async fn setup_and_run<F, Fut>(handle: F)
where
    F: FnOnce(MySqlPool) -> Fut,
    F: 'static,
    Fut: Future<Output = ()>
{
    let docker = testcontainers::clients::Cli::default();
    let image = Mysql::default()
        .with_env("MYSQL_ROOT_PASSWORD", "password");
    let service = docker.run(image);
    let host_port = service.get_host_port(3306).unwrap();
    let pool = MySqlPool::connect_with(MySqlConnectOptions::new()
        .host("127.0.0.1")
        .username("root")
        .password("password")
        .port(host_port)
        .ssl_mode(MySqlSslMode::Disabled))
        .await
        .unwrap();

    handle(pool).await;
}