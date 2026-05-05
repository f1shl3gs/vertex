use configurable::Configurable;
use event::{Metric, tags};
use serde::{Deserialize, Serialize};

use super::connection::{Connection, Error};
use super::sanitize;

const USER_QUERY: &str = "SELECT
  user,
  host,
  Select_priv,
  Insert_priv,
  Update_priv,
  Delete_priv,
  Create_priv,
  Drop_priv,
  Reload_priv,
  Shutdown_priv,
  Process_priv,
  File_priv,
  Grant_priv,
  References_priv,
  Index_priv,
  Alter_priv,
  Show_db_priv,
  Super_priv,
  Create_tmp_table_priv,
  Lock_tables_priv,
  Execute_priv,
  Repl_slave_priv,
  Repl_client_priv,
  Create_view_priv,
  Show_view_priv,
  Create_routine_priv,
  Alter_routine_priv,
  Create_user_priv,
  Event_priv,
  Trigger_priv,
  Create_tablespace_priv,
  max_questions,
  max_updates,
  max_connections,
  max_user_connections
FROM mysql.user";

#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    /// Since 5.1, Collect data from mysql.user
    #[serde(default)]
    privileges: bool,
}

pub async fn collect(conn: &mut Connection, conf: &Config) -> Result<Vec<Metric>, Error> {
    let mut rows = conn.query(USER_QUERY).await?;

    let mut metrics = vec![];
    while let Some(mut row) = rows.next().await? {
        let user = row.get_str();
        let host = row.get_str();

        if conf.privileges {
            metrics.reserve(18);

            for column in row.columns().iter().skip(2).take(18) {
                let value = match row.get_str() {
                    "Y" => true,
                    "N" => false,
                    // silently skip unparsable values
                    _ => continue,
                };

                metrics.push(Metric::gauge_with_tags(
                    format!("mysql_mysql_{}", sanitize(column.name())),
                    format!("{} by user", column.name()),
                    value,
                    tags! {"mysql_user" => user, "hostmask" => host},
                ))
            }
        } else {
            // we don't need those field
            for _ in 0..18 {
                let _ = row.get_str();
            }
        }

        let max_questions = row.get_str().parse::<u32>()?;
        let max_updates = row.get_str().parse::<u32>()?;
        let max_connections = row.get_str().parse::<u32>()?;
        let max_user_connections = row.get_str().parse::<u32>()?;

        metrics.extend([
            Metric::gauge_with_tags(
                "mysql_mysql_max_questions",
                "The number of max_questions by user.",
                max_questions,
                tags! {"mysql_user" => user, "hostmask" => host},
            ),
            Metric::gauge_with_tags(
                "mysql_mysql_max_updates",
                "The number of max_updates by user.",
                max_updates,
                tags! {"mysql_user" => user, "hostmask" => host},
            ),
            Metric::gauge_with_tags(
                "mysql_mysql_max_connections",
                "The number of max_connections by user.",
                max_connections,
                tags! {"mysql_user" => user, "hostmask" => host},
            ),
            Metric::gauge_with_tags(
                "mysql_mysql_max_user_connections",
                "The number of max_user_connections by user.",
                max_user_connections,
                tags! {"mysql_user" => user, "hostmask" => host},
            ),
        ]);
    }

    Ok(metrics)
}
