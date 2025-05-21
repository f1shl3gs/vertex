use std::time::Instant;

use resolver::{RecordClass, RecordData, RecordType, Resolver};

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let hostname = args.next().unwrap();
    let typ = args
        .next()
        .map(|s| match s.as_str() {
            "A" => RecordType::A,
            "NS" => RecordType::NS,
            "CNAME" => RecordType::CNAME,
            "SOA" => RecordType::SOA,
            "PTR" => RecordType::PTR,
            "MX" => RecordType::MX,
            "TXT" => RecordType::TXT,
            "AAAA" => RecordType::AAAA,
            "SRV" => RecordType::SRV,
            "OPT" => RecordType::OPT,
            "WKS" => RecordType::WKS,
            "HINFO" => RecordType::HINFO,
            "MINFO" => RecordType::MINFO,
            "AXFR" => RecordType::AXFR,
            "ALL" => RecordType::ALL,
            _ => panic!("unknown record type {s:?}"),
        })
        .unwrap_or(RecordType::A);
    let class = args
        .next()
        .map(|s| match s.as_str() {
            "INET" => RecordClass::INET,
            "CSNET" => RecordClass::CSNET,
            "CHAOS" => RecordClass::CHAOS,
            "HESIOD" => RecordClass::HESIOD,
            "ANY" => RecordClass::ANY,
            _ => panic!("unknown record class {s:?}"),
        })
        .unwrap_or(RecordClass::INET);

    let resolver = Resolver::with_defaults().unwrap();

    let start = Instant::now();
    let msg = resolver.lookup(&hostname, typ, class).await.unwrap();
    let elapsed = start.elapsed();

    println!(
        "opcode: {}, status: {}, id: {}",
        msg.header.opcode(),
        msg.header.response_code().as_str(),
        msg.header.id()
    );
    let mut flags = vec![];
    if msg.header.response() {
        flags.push("qr");
    }
    if msg.header.recursion_desired() {
        flags.push("rd");
    }
    if msg.header.recursion_available() {
        flags.push("ra");
    }

    println!(
        "flags: {}; QUERY: {}, ANSWER: {}, AUTHORITY: {}, ADDITIONAL: {}",
        flags.join(" "),
        msg.header.questions,
        msg.header.answers,
        msg.header.authorities,
        msg.header.additionals
    );
    println!();
    println!("QUESTION SECTION:");
    let question = msg.questions.first().unwrap();
    println!(
        "{:-32} {:-6} {:-6}\n",
        String::from_utf8_lossy(&question.name),
        match question.class {
            RecordClass::INET => "IN",
            RecordClass::CSNET => "CSNET",
            RecordClass::CHAOS => "CH",
            RecordClass::HESIOD => "HS",
            RecordClass::NONE => "NONE",
            RecordClass::ANY => "ANY",
            RecordClass::OPT(_) => "OPT",
            RecordClass::Unknown(_) => "UNKNOWN",
        },
        question.typ.as_str(),
    );

    println!("ANSWER SECTION:");
    for answer in msg.answers {
        let data = match answer.data {
            RecordData::A(ip) => ip.to_string(),
            RecordData::AAAA(ip) => ip.to_string(),
            RecordData::NoData => "NoData".to_string(),
            RecordData::CNAME(cname) => String::from_utf8_lossy(&cname).to_string(),
            _ => "NOT SUPPORTED".to_string(),
        };

        println!(
            "{:-32} {:-4} {:-6} {:-6} {:-6}",
            String::from_utf8_lossy(&answer.name),
            answer.ttl,
            match answer.class {
                RecordClass::INET => "IN",
                RecordClass::CSNET => "CSNET",
                RecordClass::CHAOS => "CH",
                RecordClass::HESIOD => "HS",
                RecordClass::NONE => "NONE",
                RecordClass::ANY => "ANY",
                RecordClass::OPT(_) => "OPT",
                RecordClass::Unknown(_) => "UNKNOWN",
            },
            answer.typ.as_str(),
            data,
        );
    }

    println!("\nQuery time: {:.04}ms", elapsed.as_secs_f64() * 1000.0);
}
