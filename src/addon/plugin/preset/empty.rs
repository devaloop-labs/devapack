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
// USAGE - Old syntax (deprecated, still supported):
//   @use publisher.name as myPlugin
//   let myProcess = myPlugin.process { gain: 0.8 }
//
// NEW SYNTAX - Chained parameters (recommended):
//   @use publisher.name as myPlugin
//   let myProcess = myPlugin.process
//       -> gain(0.8)

use std::sync::{Mutex, OnceLock};

// Plugin state
struct PluginState {
    gain: f32,
}

static STATE: OnceLock<Mutex<PluginState>> = OnceLock::new();

fn with_state<F, R>(f: F) -> R 
where 
    F: FnOnce(&mut PluginState) -> R 
{
    let m = STATE.get_or_init(|| Mutex::new(PluginState { gain: 1.0 }));
    let mut g = m.lock().unwrap();
    f(&mut *g)
}

// Export: "gain" - Set gain/volume
devalang::export_plugin!(gain, |_out, _params, _note, _freq, amp| {
    with_state(|state| {
        state.gain = amp.clamp(0.0, 1.0);
    });
});

// Export: "process" - Simple gain/volume control
devalang::export_plugin!(process, |out, _params, _note, _freq, _amp| {
    with_state(|state| {
        for sample in out.iter_mut() {
            *sample *= state.gain;
        }
    });
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
