mod triangle;

#[cfg(test)]
mod tests {
    use crate::triangle::triangle;

    #[test]
    fn triangle_test() {
        triangle();
    }
}