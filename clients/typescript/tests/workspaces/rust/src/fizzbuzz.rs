// Auxiliary file for workspace tests; not compiled by the crate.
pub fn fizzbuzz(n: i32) -> String {
    if n % 15 == 0 {
        "fizzbuzz".into()
    } else if n % 3 == 0 {
        "fizz".into()
    } else if n % 5 == 0 {
        "buzz".into()
    } else {
        n.to_string()
    }
}
