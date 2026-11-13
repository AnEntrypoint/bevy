mod bench;
mod client;
mod protocol;
mod server;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let number = |index: usize, fallback: usize| {
        args.get(index)
            .and_then(|value| value.parse().ok())
            .unwrap_or(fallback)
    };
    match args.first().map(String::as_str) {
        Some("bench") => bench::run(number(1, 2000), number(2, 300)),
        Some("det") => {
            if !bench::det(number(1, 2000), number(2, 300)) {
                std::process::exit(1);
            }
        }
        Some("server") => server::run(),
        Some("client") => client::run(),
        _ => {
            eprintln!("usage: bevy-jayzee <bench [bodies] [ticks] | det [bodies] [ticks] | server | client>");
            std::process::exit(2);
        }
    }
}
