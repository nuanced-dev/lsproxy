mod util;
pub use util::{add, Greeter};

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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fb() {
        assert_eq!(fizzbuzz(15), "fizzbuzz");
    }
}
