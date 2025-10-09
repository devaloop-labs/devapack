use crate::publisher::request::get_user_publishers;

pub async fn list_publishers() -> Result<(), String> {
    let publishers = match get_user_publishers().await {
        Ok(pubs) => pubs,
        Err(e) => {
            return Err(format!("Failed to fetch publishers: {}", e));
        }
    };

    for publisher in publishers {
        println!("- {} ({})", publisher.display_name, publisher.identifier);
    }

    Ok(())
}
