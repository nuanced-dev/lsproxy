pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub struct Greeter {
    pub name: String,
}

impl Greeter {
    pub fn new(name: &str) -> Greeter {
        Greeter {
            name: name.to_string(),
        }
    }
    pub fn greet(&self) -> String {
        format!("Hello, {}!", self.name)
    }
}
