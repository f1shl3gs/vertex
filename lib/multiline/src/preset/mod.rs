mod cri;
mod docker;
mod go;
mod java;
mod noindent;
mod regex;

// Re-exports
pub use cri::Cri;
pub use docker::Docker;
pub use go::Go;
pub use java::Java;
pub use noindent::NoIndent;
pub use regex::Regex;

#[cfg(test)]
async fn assert_rule<R: crate::aggregate::Rule>(input: &[&str], output: &[&str], rule: R) {
    use std::time::Duration;

    use bytes::Bytes;
    use futures::StreamExt;

    use super::Logic;
    use crate::LineAgg;

    let input = futures::stream::iter(
        input
            .iter()
            .map(|line| ((), Bytes::from(line.to_string()), ())),
    );
    let output = output
        .iter()
        .map(|line| Bytes::from(line.to_string()))
        .collect::<Vec<_>>();

    let logic = Logic::new(rule, Duration::from_secs(1));
    let agg = LineAgg::new(input, logic);

    let got = agg.map(|(_, data, _)| data).collect::<Vec<_>>().await;

    if got != output {
        let mut first = true;
        println!("----------------------------------- WANT ------------------------------------");
        for line in output {
            if first {
                first = false;
            } else {
                println!(
                    "-----------------------------------------------------------------------------"
                );
            }
            println!("{}", String::from_utf8_lossy(&line));
        }

        println!();

        let mut first = true;
        println!("----------------------------------- GOT -------------------------------------");
        for line in got {
            if first {
                first = false;
            } else {
                println!(
                    "-----------------------------------------------------------------------------"
                );
            }

            println!("{}", String::from_utf8_lossy(&line));
        }
    }
}
