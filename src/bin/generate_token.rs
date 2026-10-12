use std::env;
use zaakchat::auth::create_jwt;

fn main() {
    let args: Vec<String> = env::args().collect();
    let user_id = if args.len() > 1 {
        &args[1]
    } else {
        "alice@example.com"
    };

    if env::var("JWT_SECRET").map(|s| s.is_empty()).unwrap_or(true) {
        eprintln!("Warning: JWT_SECRET is not set; a server will not accept this token.");
    }

    match create_jwt(user_id) {
        Ok(token) => {
            println!("Generated JWT for user: {}", user_id);
            println!("Token: {}", token);
            println!("\nUse it in header:");
            println!("Authorization: Bearer {}", token);
        }
        Err(e) => {
            eprintln!("Failed to generate token: {}", e);
            std::process::exit(1);
        }
    }
}
