use chrono::DateTime;

// Dump checkpoints to human-readable format
fn main() {
    let path = std::env::args().nth(1).unwrap();

    let buf = std::fs::read(path).unwrap();

    // header
    let timestamp = u64::from_ne_bytes((&buf[..8]).try_into().unwrap());
    let running = u32::from_ne_bytes((&buf[8..12]).try_into().unwrap());
    let deleted = u32::from_ne_bytes((&buf[12..16]).try_into().unwrap());

    let timestamp = DateTime::from_timestamp_millis(timestamp as i64).unwrap();

    println!("{timestamp}, running: {running}, deleted: {deleted}");
    println!("running: ");

    for i in 0..(running + deleted) as usize {
        if i == running as usize {
            println!("deleted: ");
        }

        let pos = 2 * size_of::<u64>() + i * (4 * size_of::<u64>());

        let dev = u64::from_ne_bytes(buf[pos..pos + 8].try_into().unwrap());
        let inode = u64::from_ne_bytes(buf[pos + 8..pos + 16].try_into().unwrap());
        let timestamp = u64::from_ne_bytes(buf[pos + 16..pos + 24].try_into().unwrap());
        let offset = u64::from_ne_bytes(buf[pos + 24..pos + 32].try_into().unwrap());

        let timestamp = DateTime::from_timestamp_millis(timestamp as i64).unwrap();

        println!("  {timestamp} {dev}:{inode} {offset}");
    }
}
