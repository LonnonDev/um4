mod triangle;

#[cfg(test)]
mod tests {
    use crate::triangle::triangle;

    #[test]
    fn triangle_test() {
        triangle();
    }

    #[test]
    fn epic() {
        let result = "10".parse::<i32>();
    }
}