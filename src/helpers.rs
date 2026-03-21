pub fn to_natural(name: &str) -> String {
    // Regex stolen from https://stackoverflow.com/questions/3216085/split-a-pascalcase-string-into-separate-words#3216204
    let mut reconstructed = String::new();

    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() && i != 0 {
            reconstructed.push(' ');
        }
        reconstructed.push(c);
    }

    reconstructed
}

#[test]
fn test_to_natural() {
    assert_eq!(to_natural("LowPassFilter"), String::from("Low Pass Filter"));
    assert_eq!(
        to_natural("SuperDuperMegaFilterOfDoom"),
        String::from("Super Duper Mega Filter Of Doom")
    );
}
