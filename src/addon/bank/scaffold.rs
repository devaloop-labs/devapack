use std::path::Path;

/// Scaffold a new bank with the given parameters.
///
/// ### Parameters
/// - `cwd`: The current working directory.
/// - `name`: The name of the bank.
/// - `publisher`: The publisher of the bank.
/// - `description`: A brief description of the bank.
/// - `access`: The access level of the bank.
///
pub async fn scaffold_bank(
    cwd: &str,
    name: String,
    publisher: String,
    description: String,
    access: String,
) -> Result<(), String> {
    let banks_root = Path::new(cwd).join("generated").join("banks");

    let bank_path = banks_root.join(&publisher).join(&name);
    if bank_path.exists() {
        crate::utils::logger::Logger::new().log_message(
            crate::utils::logger::LogLevel::Error,
            "bank already exists, aborting",
        );
        return Err("bank already exists, aborting".into());
    }

    if let Err(e) = std::fs::create_dir_all(&bank_path) {
        crate::utils::logger::Logger::new().log_message(
            crate::utils::logger::LogLevel::Error,
            &format!("Error creating bank directory: {}", e),
        );
        return Err(format!("Failed to create bank directory: {}", e));
    }

    let audio_path = "audio/";

    if let Err(e) = create_bank_toml(
        &bank_path,
        name.as_str(),
        publisher.as_str(),
        description.as_str(),
        audio_path,
        access.as_str(),
    )
    .await
    {
        crate::utils::logger::Logger::new().log_message(
            crate::utils::logger::LogLevel::Error,
            &format!("Error creating bank toml: {}", e),
        );
        return Err(format!("Failed to create bank toml: {}", e));
    }

    if let Err(e) = create_bank_audio_dir(&bank_path).await {
        crate::utils::logger::Logger::new().log_message(
            crate::utils::logger::LogLevel::Error,
            &format!("Error creating bank audio directory: {}", e),
        );
        return Err(format!("Failed to create bank audio directory: {}", e));
    }

    if let Err(e) = write_default_docs(
        &bank_path,
        publisher.as_str(),
        name.as_str(),
        description.as_str(),
    )
    .await
    {
        crate::utils::logger::Logger::new().log_message(
            crate::utils::logger::LogLevel::Warning,
            &format!("Warning: failed to create default docs: {}", e),
        );
    }

    Ok(())
}

/// Creates the bank.toml file for the new bank.
///
/// ### Parameters
/// - `bank_path`: The path to the bank directory.
/// - `name`: The name of the bank.
/// - `publisher`: The publisher of the bank.
/// - `description`: A brief description of the bank.
/// - `audio_path`: The path to the audio directory.
/// - `access`: The access level of the bank.
///
pub async fn create_bank_toml(
    bank_path: &Path,
    name: &str,
    publisher: &str,
    description: &str,
    audio_path: &str,
    access: &str,
) -> Result<(), String> {
    let version = "0.0.1";
    let bank_toml_content = format!(
        "[bank]\nname = \"{name}\"\npublisher = \"{publisher}\"\naudio_path = \"{audio_path}\"\ndescription = \"{description}\"\nversion = \"{version}\"\naccess = \"{access}\"\n",
        name = name,
        publisher = publisher,
        audio_path = audio_path,
        description = description,
        version = version,
        access = access
    );

    if let Err(e) = std::fs::write(bank_path.join("bank.toml"), bank_toml_content) {
        eprintln!("Error creating bank.toml file: {}", e);
        return Err(format!("Failed to create bank.toml file: {}", e));
    }

    Ok(())
}

/// Writes the audio directory for the new bank.
///
/// ### Parameters
/// - `bank_path`: The path to the bank directory.
///
pub async fn create_bank_audio_dir(bank_path: &Path) -> Result<(), String> {
    let audio_dir = bank_path.join("audio");

    if let Err(e) = std::fs::create_dir_all(&audio_dir) {
        eprintln!("Error creating bank audio directory: {}", e);
        return Err(format!("Failed to create bank audio directory: {}", e));
    }

    Ok(())
}

/// Writes default documentation files (README.md and LICENSE) for the new bank.
///
/// ### Parameters
/// - `bank_path`: The path to the bank directory.
/// - `publisher`: The publisher of the bank.
/// - `name`: The name of the bank.
/// - `description`: A brief description of the bank.
///
async fn write_default_docs(
    bank_path: &Path,
    publisher: &str,
    name: &str,
    description: &str,
) -> Result<(), String> {
    // README.md
    let readme_path = bank_path.join("README.md");
    if !readme_path.exists() {
        let readme = format!(
            "# {}.{} Bank\n\n{}\n\nContents:\n- bank.toml\n- audio/ (assets)\n- LICENSE\n\nBuilt with devapack.\n",
            publisher, name, description
        );
        std::fs::write(&readme_path, readme)
            .map_err(|e| format!("Failed to write README.md: {}", e))?;
    }

    // LICENSE (MIT)
    let license_path = bank_path.join("LICENSE");
    if !license_path.exists() {
        let license = format!(
            "MIT License\n\nCopyright (c) {}\n\nPermission is hereby granted, free of charge, to any person obtaining a copy\n of this software and associated documentation files (the \"Software\"), to deal\n in the Software without restriction, including without limitation the rights\n to use, copy, modify, merge, publish, distribute, sublicense, and/or sell\n copies of the Software, and to permit persons to whom the Software is\n furnished to do so, subject to the following conditions:\n\nThe above copyright notice and this permission notice shall be included in all\n copies or substantial portions of the Software.\n\nTHE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR\n IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,\n FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE\n publisherS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER\n LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,\n OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE\n SOFTWARE.\n",
            publisher
        );
        std::fs::write(&license_path, license)
            .map_err(|e| format!("Failed to write LICENSE: {}", e))?;
    }

    Ok(())
}
