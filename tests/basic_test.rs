
#[cfg(test)]
mod tests {
    use cpal::available_hosts;
    use nokhwa::nokhwa_check;

    #[test]
    fn test_nokwa() {
        assert!(nokhwa_check());
    }
    
    #[test]
    fn test_cpal() {
        assert!(available_hosts().len() > 0);
    }
}