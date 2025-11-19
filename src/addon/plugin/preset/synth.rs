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
// USAGE - Old syntax (deprecated, still supported):
//   @use publisher.name as myPlugin
//   let synth = myPlugin.synth { waveform: "sine", gain: 0.8 }
//
// NEW SYNTAX - Chained parameters (recommended):
//   @use publisher.name as myPlugin
//   let synth = myPlugin.synth
//       -> waveform("sine")
//       -> gain(0.8)
//       -> note(C4)

use std::sync::{Mutex, OnceLock};

// Synth state shared across all invocations
struct SynthState {
    phase: f32,
    waveform: String,  // "sine", "square", "saw", "triangle"
    gain: f32,
}

static STATE: OnceLock<Mutex<SynthState>> = OnceLock::new();

fn with_state<F, R>(f: F) -> R 
where 
    F: FnOnce(&mut SynthState) -> R 
{
    let m = STATE.get_or_init(|| Mutex::new(SynthState {
        phase: 0.0,
        waveform: "sine".to_string(),
        gain: 0.8,
    }));
    let mut g = m.lock().unwrap();
    f(&mut *g)
}

// Export: "waveform" - Set waveform type
devalang::export_plugin!(waveform, |_out, _params, _note, freq, _amp| {
    // The waveform is passed as frequency (0.0 = sine, 1.0 = square, 2.0 = saw, 3.0 = triangle)
    with_state(|state| {
        state.waveform = match (freq as i32) % 4 {
            1 => "square".to_string(),
            2 => "saw".to_string(),
            3 => "triangle".to_string(),
            _ => "sine".to_string(),
        };
    });
});

// Export: "gain" - Set gain/volume
devalang::export_plugin!(gain, |_out, _params, _note, _freq, amp| {
    with_state(|state| {
        state.gain = amp.clamp(0.0, 1.0);
    });
});

// Export: "reset" - Reset phase to 0
devalang::export_plugin!(reset, |_out, _params, _note, _freq, _amp| {
    with_state(|state| state.phase = 0.0);
});

// Export: "synth" - Main synthesizer
// Generates waveform based on current state
devalang::export_plugin!(synth, |out, params, _note, freq, amp| {
    if params.sample_rate == 0 { return; }
    
    let sr = params.sample_rate as f32;
    let two_pi = 2.0 * std::f32::consts::PI;
    
    with_state(|state| {
        let step = two_pi * freq / sr;
        let effective_gain = state.gain * amp;
        
        for frame in 0..params.frames {
            // Generate sample based on waveform
            let sample = match state.waveform.as_str() {
                "square" => if state.phase < std::f32::consts::PI { 1.0 } else { -1.0 },
                "saw" => (state.phase / std::f32::consts::PI) - 1.0,
                "triangle" => {
                    let normalized = state.phase / (2.0 * std::f32::consts::PI);
                    if normalized < 0.5 {
                        4.0 * normalized - 1.0
                    } else {
                        3.0 - 4.0 * normalized
                    }
                },
                _ => state.phase.sin(),  // default sine
            };
            
            let output_sample = sample * effective_gain;
            
            // Add to all channels
            for ch in 0..params.channels {
                let idx = (frame * params.channels + ch) as usize;
                if idx < out.len() {
                    out[idx] += output_sample;
                }
            }
            
            // Advance phase
            state.phase += step;
            if state.phase >= two_pi {
                state.phase -= two_pi;
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
