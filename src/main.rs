use templating_engine::parse::*;

fn main() {
    let _ = dbg!(templating_engine::parse_simd::parse_template(
        include_bytes!("../test.txt")
    ));
    // let _ = dbg!(parse_template(include_bytes!("../test.txt")));
}
