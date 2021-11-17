use sqlx::MySqlPool;
use event::Metric;

use crate::sources::mysqld::Error;


pub async fn gather(pool: &MySqlPool) -> Result<Vec<Metric>, Error> {

}