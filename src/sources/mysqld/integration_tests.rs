use std::time::Duration;

use sqlx::mysql::{MySqlConnectOptions, MySqlSslMode};
use sqlx::MySqlPool;

use super::{
    get_mysql_version, global_status, global_variables, info_schema_innodb_cmp,
    info_schema_innodb_cmpmem, info_schema_query_response_time, slave_status,
};
use crate::testing::{ContainerBuilder, WaitFor};

#[tokio::test]
async fn gather() {
    // The official MySQL image does not contains response_time plugin,
    // while percona provide it
    let container = ContainerBuilder::new("percona:5.7.35")
        .with_env("MYSQL_ROOT_PASSWORD", "password")
        .with_port(3306)
        .run()
        .unwrap();
    container
        .wait(WaitFor::Stderr("ready for connections"))
        .unwrap();
    let addr = container.get_mapped_addr(3306);

    tokio::time::sleep(Duration::from_secs(15)).await;

    let pool = MySqlPool::connect_with(
        MySqlConnectOptions::new()
            .host("127.0.0.1")
            .username("root")
            .password("password")
            .port(addr.port())
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
        sqlx::query(q).execute(&pool).await.unwrap();
    }
    info_schema_query_response_time::gather(&pool)
        .await
        .unwrap();
}
