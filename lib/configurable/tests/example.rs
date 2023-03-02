#![allow(clippy::print_stdout)]

use std::time::Duration;

use configurable::example::Visitor;
use configurable::schema::generate_root_schema;
use configurable_derive::Configurable;
use serde::{Deserialize, Serialize};

#[test]
fn generate_example() {
    #[derive(Configurable, Clone, Debug, Deserialize, Serialize, Default)]
    pub struct Sub {
        /// offset
        #[configurable(example = "-8")]
        offset: String,
    }

    fn default_timeout() -> Duration {
        Duration::from_secs(10)
    }

    fn default_sub() -> Sub {
        Sub {
            offset: "ssss".to_string(),
        }
    }

    /// line 1
    /// line 2
    #[derive(Clone, Debug, Configurable, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    pub struct NtpConfig {
        /// empty arr desc
        u32_arr: Vec<u32>,

        /// struct array of Sub
        struct_arr: Vec<Sub>,

        /// sub desc
        #[serde(default = "default_sub")]
        sub: Sub,

        /// Time for NTP round-trip. in seconds.
        ///
        /// blah.
        /// sss
        ///
        /// xxx
        #[serde(default = "default_timeout")]
        #[serde(with = "humanize::duration::serde")]
        pub timeout: Duration,

        /// Address for NTP client to connect
        #[configurable(format = "hostname", example = "pool.ntp.org")]
        pub pools: Vec<String>,
    }

    let root_schema = generate_root_schema::<NtpConfig>().unwrap();

    let text = serde_json::to_string_pretty(&root_schema).unwrap();
    println!("{}", text);

    let visitor = Visitor::new(root_schema);
    let example = visitor.example();

    println!("{}", example)
}

#[test]
fn ser() {
    let d = Duration::from_secs(10);
    let n = humanize::duration::serde::serialize(&d, serde_json::value::Serializer).unwrap();
    println!("{}", n);
}
