use resolver::Resolver;

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let hostname = args.next().unwrap();

    let resolver = Resolver::with_defaults().unwrap();

    let lookup = resolver.lookup_ipv4(hostname.as_str()).await.unwrap();
    for addr in lookup {
        println!("{addr:?}");
    }
}
