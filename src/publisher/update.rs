use crate::{
    publisher::request::post_update_publisher_to_forge_api,
    types::publisher::PublisherInfoUpdate,
    utils::{logger::Logger, spinner::with_spinner},
};

pub async fn prompt_update_publisher(name: Option<String>) -> Result<(), String> {
    println!();
    println!("⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯");
    println!("Devalang Publisher Updater");
    println!("⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯");
    println!();

    let user_publishers = match crate::publisher::request::get_user_publishers().await {
        Ok(publishers) => publishers,
        Err(e) => {
            return Err(format!("Failed to fetch user publishers: {}", e));
        }
    };

    let users_publisher_names: Vec<String> = user_publishers
        .iter()
        .map(|p| format!("{} ({})", p.identifier, p.display_name.clone()))
        .collect();

    // Determine selected publisher identifier and its full record
    let selected_identifier: String = if let Some(name_str) = name {
        // if the provided name matches an identifier, use it; otherwise prompt
        if user_publishers.iter().any(|p| p.identifier == name_str) {
            name_str
        } else {
            match inquire::Select::new("Select a publisher to update:", users_publisher_names)
                .prompt()
            {
                Ok(label) => {
                    // label format is "<identifier> (<display_name>)" -> extract identifier
                    label.split(" (").next().unwrap_or(&label).to_string()
                }
                Err(e) => {
                    return Err(format!("Failed to prompt for publisher selection: {}", e));
                }
            }
        }
    } else {
        match inquire::Select::new("Select a publisher to update:", users_publisher_names).prompt()
        {
            Ok(label) => label.split(" (").next().unwrap_or(&label).to_string(),
            Err(e) => {
                return Err(format!("Failed to prompt for publisher selection: {}", e));
            }
        }
    };

    let selected_idx = user_publishers
        .iter()
        .position(|p| p.identifier == selected_identifier)
        .ok_or_else(|| format!("Selected publisher not found: {}", selected_identifier))?;
    let current = &user_publishers[selected_idx];

    let default_display = current.display_name.clone();
    let mut display_name = match inquire::Text::new("Enter the publisher display name:")
        .with_default(&default_display)
        .prompt()
    {
        Ok(s) => s.to_string(),
        Err(e) => {
            return Err(format!(
                "Failed to prompt for publisher display name: {}",
                e
            ));
        }
    };
    if display_name.trim().is_empty() {
        display_name = default_display;
    }

    let default_description = current.description.clone();
    let mut description = match inquire::Text::new("Enter the publisher description:")
        .with_default(&default_description)
        .prompt()
    {
        Ok(s) => s.to_string(),
        Err(e) => {
            return Err(format!("Failed to prompt for publisher description: {}", e));
        }
    };
    if description.trim().is_empty() {
        description = default_description;
    }

    let default_logo = current.logo_url.clone().unwrap_or_default();
    let logo_input = match inquire::Text::new("Enter the publisher logo URL (optional):")
        .with_default(&default_logo)
        .prompt()
    {
        Ok(s) => s.to_string(),
        Err(e) => {
            return Err(format!("Failed to prompt for publisher logo URL: {}", e));
        }
    };
    let logo_url = if logo_input.trim().is_empty() {
        current.logo_url.clone()
    } else {
        Some(logo_input)
    };

    let default_banner = current.banner_url.clone().unwrap_or_default();
    let banner_input = match inquire::Text::new("Enter the publisher banner URL (optional):")
        .with_default(&default_banner)
        .prompt()
    {
        Ok(s) => s.to_string(),
        Err(e) => {
            return Err(format!("Failed to prompt for publisher banner URL: {}", e));
        }
    };
    let banner_url = if banner_input.trim().is_empty() {
        current.banner_url.clone()
    } else {
        Some(banner_input)
    };

    let default_country = current.country_code.clone().unwrap_or_default();
    let country_input =
        match inquire::Text::new("Enter the publisher country code (e.g., US, GB, FR) (optional):")
            .with_default(&default_country)
            .prompt()
        {
            Ok(s) => s.to_string(),
            Err(e) => {
                return Err(format!(
                    "Failed to prompt for publisher country code: {}",
                    e
                ));
            }
        };
    let country_code = if country_input.trim().is_empty() {
        current.country_code.clone().unwrap_or_default()
    } else {
        country_input
    };

    let default_tags = current.tags.join(", ");
    let tags_input =
        match inquire::Text::new("Enter tags for the publisher (comma-separated, optional):")
            .with_default(&default_tags)
            .prompt()
        {
            Ok(s) => s.to_string(),
            Err(e) => {
                return Err(format!("Failed to prompt for publisher tags: {}", e));
            }
        };
    let tags = if tags_input.trim().is_empty() {
        current.tags.clone()
    } else {
        tags_input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };

    let publisher_payload = PublisherInfoUpdate {
        display_name,
        description,
        logo_url,
        banner_url,
        country_code: if country_code.trim().is_empty() {
            None
        } else {
            Some(country_code)
        },
        tags,
    };

    println!();
    println!("⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯");
    println!("Confirm Publisher Update");
    println!("⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯");
    println!();

    println!("Identifier   : {}", &selected_identifier);
    println!("Display Name : {}", &publisher_payload.display_name);
    println!("Description  : {}", &publisher_payload.description);
    println!(
        "Logo URL     : {}",
        &publisher_payload
            .logo_url
            .as_deref()
            .unwrap_or("Not provided")
    );
    println!(
        "Banner URL   : {}",
        &publisher_payload
            .banner_url
            .as_deref()
            .unwrap_or("Not provided")
    );
    println!(
        "Country Code : {}",
        &publisher_payload
            .country_code
            .as_deref()
            .unwrap_or("Not provided")
    );
    println!(
        "Tags         : {}",
        if publisher_payload.tags.is_empty() {
            "None".to_string()
        } else {
            publisher_payload.tags.join(", ")
        }
    );
    println!();

    let confirm =
        match inquire::Confirm::new("Are all details correct? (this will update the publisher)")
            .with_default(true)
            .prompt()
        {
            Ok(confirm) => confirm,
            Err(e) => {
                return Err(format!("Failed to prompt for confirmation: {}", e));
            }
        };

    if !confirm {
        println!("Publisher update cancelled by user.");
        return Ok(());
    }

    let update_spinner = with_spinner("Updating publisher...");

    if let Err(e) =
        post_update_publisher_to_forge_api(&selected_identifier, &publisher_payload).await
    {
        return Err(format!("Failed to update publisher: {}", e));
    }

    update_spinner.finish_and_clear();

    let logger = Logger::new();
    logger.log_message(
        crate::utils::logger::LogLevel::Success,
        "Publisher updated successfully!",
    );

    Ok(())
}
