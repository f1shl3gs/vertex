use std::io::Write;
use std::num::NonZeroU32;

use argh::FromArgs;
use governor::{Quota, RateLimiter};

#[derive(FromArgs)]
#[argh(description = "A log generator for testing or benching Vertex")]
struct Options {
    #[argh(option, short = 'r', description = "how many logs produced in 1 sec")]
    rate: u32,

    #[argh(option, short = 'f', description = "file name")]
    file: String,

    #[argh(option, short = 'l', description = "log message")]
    line: String,
}

fn main() {
    let opt: Options = argh::from_env();
    let limiter = RateLimiter::direct(Quota::per_second(NonZeroU32::new(opt.rate).unwrap()));
    let line = format!("{}\n", opt.line);

    let mut writer = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(opt.file)
        .expect("Open file failed");

    loop {
        match limiter.check() {
            Ok(_) => {
                let _written = writer.write(line.as_bytes()).expect("Write failed");
            }
            Err(not_until) => {
                let possible = not_until.earliest_possible();
                let wait = not_until.wait_time_from(possible);
                std::thread::sleep(wait);
            }
        }
    }
}
