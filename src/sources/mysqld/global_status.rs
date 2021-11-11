use sqlx::MySqlPool;

pub async fn query(pool: &MySqlPool) {
    let result = sqlx::query(r#"SHOW GLOBAL STATUS"#)
        .execute(pool)
        .await
        .unwrap();


}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_query() {
        // username:password@protocol(address)/dbname?param=value
        let pool = MySqlPool::connect("root:password@tcp(127.0.0.1:3306)/").await.unwrap();
        let result = query(&pool).await;
    }
}