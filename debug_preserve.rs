use translatex::core::preserve::Preserver;
fn main() {
    let mut p = Preserver::new(&[
        r"[^]*".to_string(),
        r"\S+".to_string(),
    ]).unwrap().unwrap();
    let masked = p.mask("keep  b word");
    println!("masked: '{}'", masked);
    println!("tokens: {:?}", p.tokens);
    let restored = p.restore(&masked);
    println!("restored: {:?}", restored);
}
