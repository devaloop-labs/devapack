use crate::utils::logger::{LogLevel, Logger};
use crate::{
    addon::plugin::scaffold::scaffold_plugin,
    utils::{kebab_case::to_kebab_case, spinner::with_spinner},
};

pub async fn prompt_plugin_addon(cwd: &str) -> Result<(), String> {
    println!();
    println!("⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯");
    println!("Devalang Plugin Packager");
    println!("⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯");
    println!();

    let type_options = vec![
        "empty", "synth", // "fx", "sequencer", "midi", "utility"
    ];
    let final_type =
        match inquire::Select::new("Enter the plugin preset type:", type_options).prompt() {
            Ok(type_) => to_kebab_case(type_),
            Err(e) => {
                return Err(format!("Failed to prompt for plugin preset type: {}", e));
            }
        };

    let final_name = match inquire::Text::new("Enter the plugin name:")
        .with_default("myplugin")
        .prompt()
    {
        Ok(name) => to_kebab_case(&name).replace("-", ""),
        Err(e) => {
            return Err(format!("Failed to prompt for plugin name: {}", e));
        }
    };

    let final_publisher = match inquire::Text::new("Enter the plugin publisher:")
        .with_default("johndoe")
        .prompt()
    {
        Ok(publisher) => to_kebab_case(&publisher),
        Err(e) => {
            return Err(format!("Failed to prompt for plugin publisher: {}", e));
        }
    };

    let final_description = match inquire::Text::new("Enter the plugin description:")
        .with_default("A description of my plugin")
        .prompt()
    {
        Ok(description) => to_kebab_case(&description),
        Err(e) => {
            return Err(format!("Failed to prompt for plugin description: {}", e));
        }
    };

    // TODO Enable this when we support private/protected plugins
    // let options = vec!["public", "private", "protected"];
    // let final_access = match
    //     inquire::Select
    //         ::new("Select the plugin access level:", options)
    //         .with_help_message(
    //             "Select if the plugin should be public (free), private (for you only), or protected (purchased by others)."
    //         )
    //         .prompt()
    // {
    //     Ok(access) => to_kebab_case(access),
    //     Err(e) => {
    //         return Err(format!("Failed to prompt for plugin access level: {}", e));
    //     }
    // };

    let final_access = "public".to_string();

    println!();
    println!("⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯");
    println!("Confirm Plugin Details");
    println!("⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯");
    println!();

    Logger::new().log_message(LogLevel::Info, &format!("Name: {}", final_name));
    Logger::new().log_message(LogLevel::Info, &format!("Type: {}", final_type));
    Logger::new().log_message(LogLevel::Info, &format!("publisher: {}", final_publisher));
    Logger::new().log_message(
        LogLevel::Info,
        &format!("Description: {}", final_description),
    );
    Logger::new().log_message(LogLevel::Info, &format!("Access Level: {}", final_access));

    println!();

    let confirm_prompt = inquire::Confirm::new("Are these details correct ?")
        .with_default(true)
        .prompt();

    match confirm_prompt {
        Ok(true) => {
            let spinner = with_spinner("Generating plugin...");

            let res = scaffold_plugin(
                cwd,
                final_name,
                final_publisher,
                final_description,
                final_access,
                final_type,
            )
            .await;
            spinner.finish_and_clear();
            res
        }
        _ => {
            Logger::new().log_message(LogLevel::Warning, "Aborting plugin scaffolding.");
            Err("aborted by user".into())
        }
    }
}
