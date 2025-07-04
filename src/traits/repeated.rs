use colored::ColoredString;

pub trait Repeated {
    fn repeated(&self, times: usize) -> String;
}

impl Repeated for &str {
    fn repeated(&self, times: usize) -> String {
        self.repeat(times)
    }
}

impl Repeated for ColoredString {
    fn repeated(&self, times: usize) -> String {
        let colored_str = self.clone();
        let mut result = String::new();
        for _ in 0..times {
            result.push_str(&colored_str.to_string());
        }
        result
    }
}
