pub fn to_kebab_case(s: &str) -> String {
    let mut out = String::new();

    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                out.push('-');
            }
            for lower in c.to_lowercase() {
                out.push(lower);
            }
        } else {
            out.push(c);
        }
    }

    out
}
