mod global_status;
mod global_variables;
mod slave_status;
mod info_schema_innodb_cmp;
mod info_schema_innodb_cmpmem;
mod info_schema_query_response_time;

use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use crate::config::{GenerateConfig, SourceDescription, default_false, default_true};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InfoSchemaConfig {
    // Since 5.5, Collect InnoDB compressed tables metrics from information_schema.innodb_cmp.
    #[serde(default = "default_true")]
    innodb_cmp: bool,
    // Since 5.5, Collect InnoDB buffer pool compression metrics from information_schema.innodb_cmpmem.
    #[serde(default = "default_true")]
    innodb_cmpmem: bool,
    // Since 5.5, Collect query response time distribution if query_response_time_stats is ON.
    #[serde(default = "default_true")]
    query_response_time: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MysqldConfig {
    // Since 5.1, Collect from SHOW GLOBAL STATUS (Enabled by default)
    #[serde(default = "default_true")]
    global_status: bool,
    // Since 5.1, Collect from SHOW GLOBAL VARIABLES (Enabled by default)
    #[serde(default = "default_true")]
    global_variables: bool,
    // Since 5.1, Collect from SHOW SLAVE STATUS (Enabled by default)
    #[serde(default = "default_true")]
    slave_status: bool,

    // Since 5.1, collect auto_increment columns and max values from information_schema.
    #[serde(default = "default_false")]
    auto_increment_columns: bool,
    // Since 5.1, collect the current size of all registered binlog files
    #[serde(default = "default_false")]
    binlog_size: bool,

    info_schema: InfoSchemaConfig,
}

impl GenerateConfig for MysqldConfig {
    fn generate_config() -> Value {
        serde_yaml::to_value(
            Self {
                global_status: default_true(),
                global_variables: default_true(),
                slave_status: default_true(),
                auto_increment_columns: default_false(),
                binlog_size: default_false(),
                info_schema: InfoSchemaConfig {
                    innodb_cmp: default_true(),
                    innodb_cmpmem: default_true(),
                    query_response_time: default_true(),
                },
            }
        ).unwrap()
    }
}

inventory::submit! {
    SourceDescription::new::<MysqldConfig>("mysqld")
}



#[cfg(test)]
mod tests {
    use sqlx::MySqlPool;
    use super::*;

    #[tokio::test]
    async fn test_connect() {
        let pool = MySqlPool::connect("127.0.0.1:5379").await.unwrap();
    }
}