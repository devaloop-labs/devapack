use std::path::Path;

pub async fn create_plugin_src_synth(src_path: &Path) -> Result<(), String> {
    if let Err(e) = std::fs::create_dir_all(src_path) {
        crate::utils::logger::Logger::new().log_message(
            crate::utils::logger::LogLevel::Error,
            &format!("Error creating plugin src directory: {}", e),
        );
        return Err(format!("Failed to create plugin src directory: {}", e));
    }

    if let Err(e) = create_plugin_synth_src_lib(src_path).await {
        crate::utils::logger::Logger::new().log_message(
            crate::utils::logger::LogLevel::Error,
            &format!("Error creating plugin src/lib.rs: {}", e),
        );
        return Err(format!("Failed to create plugin src/lib.rs: {}", e));
    }

    Ok(())
}

async fn create_plugin_synth_src_lib(rs_path: &Path) -> Result<(), String> {
    let lib_path = rs_path.join("lib.rs");
    let src_lib_content: &'static str = r#"// Simple synth plugin using Devalang's safe plugin API
// No unsafe code, no manual FFI!
//
// Usage in devalang scripts:
// @use publisher.name as myPlugin
// let reset = myPlugin.reset
// let synth = myPlugin.synth

use std::sync::{Mutex, OnceLock};

static PHASE: OnceLock<Mutex<f32>> = OnceLock::new();

fn with_phase<F, R>(f: F) -> R 
where 
    F: FnOnce(&mut f32) -> R 
{
    let m = PHASE.get_or_init(|| Mutex::new(0.0));
    let mut g = m.lock().unwrap();
    f(&mut *g)
}

// Export: "reset" - Reset phase to 0
devalang::export_plugin!(reset, |_out, _params, _note, _freq, _amp| {
    with_phase(|p| *p = 0.0);
});

// Export: "synth" - Simple sine wave synthesizer that ADDS to the buffer
devalang::export_plugin!(synth, |out, params, _note, freq, amp| {
    if params.sample_rate == 0 { return; }
    
    let sr = params.sample_rate as f32;
    let two_pi = 2.0 * std::f32::consts::PI;
    
    with_phase(|phase| {
        let step = two_pi * freq / sr;
        
        for frame in 0..params.frames {
            let sample = phase.sin() * amp;
            
            // Add to all channels
            for ch in 0..params.channels {
                let idx = (frame * params.channels + ch) as usize;
                if idx < out.len() {
                    out[idx] += sample;
                }
            }
            
            *phase += step;
            if *phase >= two_pi {
                *phase -= two_pi;
            }
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
