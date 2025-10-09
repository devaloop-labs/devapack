use crate::{
    publisher::request::post_create_publisher_to_forge_api,
    types::publisher::PublisherInfo,
    utils::{logger::Logger, spinner::with_spinner},
};

pub async fn prompt_create_publisher() -> Result<(), String> {
    println!();
    println!("⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯");
    println!("Devalang Publisher Creator");
    println!("⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯");
    println!();

    let identifier = match inquire::Text::new("Enter the publisher identifier:")
        .with_default("mypublisher")
        .prompt()
    {
        Ok(identifier) => identifier.to_string(),
        Err(e) => {
            return Err(format!("Failed to prompt for publisher identifier: {}", e));
        }
    };

    let display_name = match inquire::Text::new("Enter the publisher display name:")
        .with_default("My Publisher")
        .prompt()
    {
        Ok(display_name) => display_name.to_string(),
        Err(e) => {
            return Err(format!(
                "Failed to prompt for publisher display name: {}",
                e
            ));
        }
    };

    let description = match inquire::Text::new("Enter the publisher description:")
        .with_default("A description of my publisher")
        .prompt()
    {
        Ok(description) => description.to_string(),
        Err(e) => {
            return Err(format!("Failed to prompt for publisher description: {}", e));
        }
    };

    let logo_url = match inquire::Text::new("Enter the publisher logo URL (optional):")
        .with_default("")
        .prompt()
    {
        Ok(logo_url) => {
            let url = logo_url.to_string();
            if url.trim().is_empty() {
                None
            } else {
                Some(url)
            }
        }
        Err(e) => {
            return Err(format!("Failed to prompt for publisher logo URL: {}", e));
        }
    };

    let banner_url = match inquire::Text::new("Enter the publisher banner URL (optional):")
        .with_default("")
        .prompt()
    {
        Ok(banner_url) => {
            let url = banner_url.to_string();
            if url.trim().is_empty() {
                None
            } else {
                Some(url)
            }
        }
        Err(e) => {
            return Err(format!("Failed to prompt for publisher banner URL: {}", e));
        }
    };

    let country_code =
        match inquire::Text::new("Enter the publisher country code (e.g., US, GB, FR) (optional):")
            .with_default("")
            .prompt()
        {
            Ok(country_code) => country_code.to_string(),
            Err(e) => {
                return Err(format!(
                    "Failed to prompt for publisher country code: {}",
                    e
                ));
            }
        };

    let tags = match inquire::Text::new("Enter tags for the publisher (comma-separated, optional):")
        .with_default("")
        .prompt()
    {
        Ok(tags) => {
            let input = tags.to_string();
            if input.trim().is_empty() {
                Vec::new()
            } else {
                input
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            }
        }
        Err(e) => {
            return Err(format!("Failed to prompt for publisher tags: {}", e));
        }
    };

    let publisher_payload = PublisherInfo {
        identifier,
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
    println!("Confirm Publisher Details");
    println!("⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯");
    println!();

    println!("Identifier   : {}", &publisher_payload.identifier);
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

    let confirm = match inquire::Confirm::new("Are all details correct?")
        .with_default(true)
        .prompt()
    {
        Ok(confirm) => confirm,
        Err(e) => {
            return Err(format!("Failed to prompt for confirmation: {}", e));
        }
    };

    if !confirm {
        println!("Publisher creation cancelled by user.");
        return Ok(());
    }

    let create_publisher_spinner = with_spinner("Creating publisher...");

    if let Err(e) = post_create_publisher_to_forge_api(&publisher_payload).await {
        return Err(format!("Failed to create publisher: {}", e));
    }

    create_publisher_spinner.finish_and_clear();

    let logger = Logger::new();
    logger.log_message(
        crate::utils::logger::LogLevel::Success,
        "Publisher created successfully!",
    );

    Ok(())
}
