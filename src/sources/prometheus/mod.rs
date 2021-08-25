use chrono;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize() {
        let mut d = chrono::Duration::minutes(2) + chrono::Duration::seconds(15);

        println!("{:?}", d.to_string());
    }
}