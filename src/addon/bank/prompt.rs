use crate::utils::logger::{LogLevel, Logger};
use crate::{
    addon::bank::scaffold::scaffold_bank,
    utils::{kebab_case::to_kebab_case, spinner::with_spinner},
};

/// Prompts the user for bank details and creates a new bank.
///
/// ### Parameters
/// - `cwd`: The current directory
///
pub async fn prompt_bank_addon(cwd: &str) -> Result<(), String> {
    println!();
    println!("⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯");
    println!("Devalang Bank Packager");
    println!("⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯");
    println!();

    let final_name = match inquire::Text::new("Enter the bank name:")
        .with_default("mybank")
        .prompt()
    {
        Ok(name) => to_kebab_case(&name).replace("-", ""),
        Err(e) => {
            return Err(format!("Failed to prompt for bank name: {}", e));
        }
    };

    let final_publisher = match inquire::Text::new("Enter the bank publisher:")
        .with_default("johndoe")
        .prompt()
    {
        Ok(publisher) => to_kebab_case(&publisher),
        Err(e) => {
            return Err(format!("Failed to prompt for bank publisher: {}", e));
        }
    };

    let final_description = match inquire::Text::new("Enter the bank description:")
        .with_default("A description of my bank")
        .prompt()
    {
        Ok(description) => description.to_string(),
        Err(e) => {
            return Err(format!("Failed to prompt for bank description: {}", e));
        }
    };

    // TODO Enable this when we support private/protected banks
    // let options = vec!["public", "private", "protected"];
    // let final_access = match
    //     inquire::Select
    //         ::new("Select the bank access level:", options)
    //         .with_help_message(
    //             "Select if the bank should be public (free), private (for you only), or protected (purchased by others)."
    //         )
    //         .prompt()
    // {
    //     Ok(access) => to_kebab_case(access),
    //     Err(e) => {
    //         return Err(format!("Failed to prompt for bank access level: {}", e));
    //     }
    // };

    let final_access = "public".to_string();

    println!();
    println!("⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯");
    println!("Confirm Bank Details");
    println!("⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯");
    println!();

    Logger::new().log_message(LogLevel::Info, &format!("Name: {}", final_name));
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
            let spinner = with_spinner("Generating bank...");

            let res = scaffold_bank(
                cwd,
                final_name,
                final_publisher,
                final_description,
                final_access,
            )
            .await;
            spinner.finish_and_clear();
            res
        }
        _ => {
            Logger::new().log_message(LogLevel::Warning, "Aborting bank scaffolding.");
            Err("aborted by user".into())
        }
    }
}
