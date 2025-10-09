use std::path::Path;

pub async fn create_plugin_src_empty(src_path: &Path) -> Result<(), String> {
    if let Err(e) = std::fs::create_dir_all(src_path) {
        crate::utils::logger::Logger::new().log_message(
            crate::utils::logger::LogLevel::Error,
            &format!("Error creating plugin src directory: {}", e),
        );
        return Err(format!("Failed to create plugin src directory: {}", e));
    }

    if let Err(e) = create_plugin_empty_src_lib(src_path).await {
        crate::utils::logger::Logger::new().log_message(
            crate::utils::logger::LogLevel::Error,
            &format!("Error creating plugin src/lib.rs: {}", e),
        );
        return Err(format!("Failed to create plugin src/lib.rs: {}", e));
    }

    Ok(())
}

async fn create_plugin_empty_src_lib(rs_path: &Path) -> Result<(), String> {
    let lib_path = rs_path.join("lib.rs");
    let src_lib_content: &'static str = r#"// Empty plugin preset using Devalang's safe plugin API
// No unsafe code needed!
//
// Usage in devalang scripts:
// @use publisher.name as myPlugin
// let myProcess = myPlugin.process

// Export: "process" - Simple gain/volume control
devalang::export_plugin!(process, |out, _params, _note, _freq, amp| {
    for sample in out.iter_mut() {
        *sample *= amp;
    }
});
"#;

    if let Err(e) = std::fs::write(&lib_path, src_lib_content) {
        crate::utils::logger::Logger::new().log_message(
            crate::utils::logger::LogLevel::Error,
            &format!("Error creating lib.rs: {}", e),
        );
        return Err(format!("Failed to create lib.rs: {}", e));
    }

    Ok(())
}
