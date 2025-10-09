pub fn compute_bump(current: &str, bump: &str) -> Result<String, String> {
    // Expect semver base 'x.y.z' (ignore any pre-release suffix when bumping)
    let base = current.split_once('-').map(|(b, _)| b).unwrap_or(current);
    let mut parts = base
        .split('.')
        .map(|s| s.parse::<u64>().unwrap_or(0))
        .collect::<Vec<_>>();
    while parts.len() < 3 {
        parts.push(0);
    }

    match bump.to_ascii_lowercase().as_str() {
        "major" => {
            parts[0] = parts[0].saturating_add(1);
            parts[1] = 0;
            parts[2] = 0;
            Ok(format!("{}.{}.{}", parts[0], parts[1], parts[2]))
        }
        "minor" => {
            parts[1] = parts[1].saturating_add(1);
            parts[2] = 0;
            Ok(format!("{}.{}.{}", parts[0], parts[1], parts[2]))
        }
        "patch" => {
            parts[2] = parts[2].saturating_add(1);
            Ok(format!("{}.{}.{}", parts[0], parts[1], parts[2]))
        }
        other => Err(format!(
            "Unknown bump type: {} (expected: major|minor|patch)",
            other
        )),
    }
}
