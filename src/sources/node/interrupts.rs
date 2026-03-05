use std::path::PathBuf;

use event::{Metric, tags};

use super::{Error, read_string};

pub async fn collect(proc_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let path = proc_path.join("interrupts");
    let content = read_string(path)?;

    let mut metrics = Vec::new();
    for interrupt in parse_interrupts(&content)? {
        metrics.extend(
            interrupt
                .values
                .into_iter()
                .enumerate()
                .map(|(cpu, value)| {
                    Metric::sum_with_tags(
                        "node_interrupts_total",
                        "Interrupt details.",
                        value,
                        tags!(
                            "cpu" => cpu,
                            "type" => interrupt.name,
                            "info" => interrupt.info,
                            "devices" => interrupt.devices,
                        ),
                    )
                }),
        );
    }

    Ok(metrics)
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
struct Interrupt<'a> {
    name: &'a str,
    info: &'a str,
    devices: &'a str,
    values: Vec<f64>,
}

fn parse_interrupts(content: &str) -> Result<Vec<Interrupt<'_>>, Error> {
    let mut lines = content.lines();

    let Some(header) = lines.next() else {
        return Err(Error::Malformed("empty interrupts"));
    };
    let cpus = header.split_ascii_whitespace().count();

    let mut interrupts = Vec::new();
    for line in content.lines() {
        let Some((name, remaining)) = line.split_once(':') else {
            continue;
        };

        let name = name.trim();
        let mut parts = remaining.split_ascii_whitespace();
        let values = parts
            .by_ref()
            .map(|value| value.parse::<f64>())
            .take(cpus)
            .collect::<Result<Vec<_>, _>>()?;

        // skip ERR and MIS line, or any malformed line
        if values.len() != cpus {
            continue;
        }

        let (info, devices) = if name.parse::<u32>().is_ok() {
            match parts.next() {
                Some(info) => {
                    let devices = remaining.split_once(info).unwrap().1.trim();
                    (info, devices)
                }
                None => ("", ""),
            }
        } else {
            let info = match parts.next() {
                Some(p) => {
                    let start = remaining.find(p).unwrap();
                    remaining.split_at(start).1.trim()
                }
                None => "",
            };

            (info, "")
        };

        interrupts.push(Interrupt {
            name,
            info,
            devices,
            values,
        })
    }

    Ok(interrupts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let content = std::fs::read_to_string("tests/node/fixtures/proc/interrupts").unwrap();
        let interrupts = parse_interrupts(&content).unwrap();

        assert_eq!(
            Interrupt {
                name: "NMI",
                info: "Non-maskable interrupts",
                devices: "",
                values: vec![47.0, 5031.0, 6211.0, 4968.0],
            },
            interrupts[15]
        );
        assert_eq!(
            Interrupt {
                name: "0",
                info: "IR-IO-APIC-edge",
                devices: "timer",
                values: vec![18.0, 0.0, 0.0, 0.0],
            },
            interrupts[0]
        )
    }
}
