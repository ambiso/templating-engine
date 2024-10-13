use templating_engine::parse::*;

fn main() {
    let _ = dbg!(parse_template(b"{{ hello }} world".as_slice()));
}
