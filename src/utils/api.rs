pub fn get_forge_api_base_url() -> String {
    std::env::var("DEVALANG_FORGE_API_URL")
        .unwrap_or_else(|_| "https://forge.devalang.com".to_string())
}
