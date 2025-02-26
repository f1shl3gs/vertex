use std::time::Duration;

use sqlx::mysql::{MySqlConnectOptions, MySqlSslMode};
use sqlx::{Error, MySqlPool};
use testify::container::Container;
use testify::next_addr;

use super::{
    get_mysql_version, global_status, global_variables, info_schema_innodb_cmp,
    info_schema_innodb_cmpmem, info_schema_query_response_time, slave_status,
};
use crate::testing::trace_init;

#[tokio::test]
async fn gather() {
    trace_init();

    let service_addr = next_addr();

    Container::new("mysql", "5.7.44")
        .with_env("MYSQL_ROOT_PASSWORD", "password")
        .with_tcp(3306, service_addr.port())
        .with_memory_limit(4 * 1024 * 1024 * 1024) // 4G
        .tail_logs(true, true)
        .run(async move {
            tokio::time::sleep(Duration::from_secs(15)).await;

            let pool = MySqlPool::connect_with(
                MySqlConnectOptions::new()
                    .host("127.0.0.1")
                    .username("root")
                    .password("password")
                    .port(service_addr.port())
                    .ssl_mode(MySqlSslMode::Disabled),
            )
            .await
            .unwrap();

            get_mysql_version(&pool).await.unwrap();
            global_variables::gather(&pool).await.unwrap();
            global_status::gather(&pool).await.unwrap();
            info_schema_innodb_cmp::gather(&pool).await.unwrap();
            info_schema_innodb_cmpmem::gather(&pool).await.unwrap();

            // TODO: set the cluster properly and test it
            //      This gather will do nothing if replication is not setup
            slave_status::gather(&pool).await.unwrap();

            // By default, the plugin is not enabled, so nothing will be gathered,
            // and there should not be any error here, so here it is
            info_schema_query_response_time::gather(&pool)
                .await
                .unwrap();

            // Now, setup the plugin and gather again
            for q in [
                r#"INSTALL PLUGIN QUERY_RESPONSE_TIME_AUDIT SONAME 'query_response_time.so'"#,
                r#"INSTALL PLUGIN QUERY_RESPONSE_TIME SONAME 'query_response_time.so'"#,
                r#"INSTALL PLUGIN QUERY_RESPONSE_TIME_READ SONAME 'query_response_time.so'"#,
                r#"INSTALL PLUGIN QUERY_RESPONSE_TIME_WRITE SONAME 'query_response_time.so'"#,
                r#"SET GLOBAL query_response_time_stats = on"#,
            ] {
                match sqlx::query(q).execute(&pool).await {
                    Ok(_result) => {}
                    Err(err) => match err {
                        Error::Database(err) => {
                            match err.code() {
                                Some(code) => {
                                    if code == "HY000" {
                                        // some version of mysql is not enabled by default,
                                        // percona does
                                    } else {
                                        panic!(
                                            "Incorrect error code returned when querying: {}",
                                            err
                                        );
                                    }
                                }
                                None => panic!("unknown database error: {:?}", err),
                            }
                        }
                        err => panic!("unexpected error: {:?}", err),
                    },
                }
            }
            info_schema_query_response_time::gather(&pool)
                .await
                .unwrap();
        })
        .await;
}
