use rspotify::{
    model::{AdditionalType, Country, Market},
    prelude::*,
    scopes, AuthCodeSpotify, Credentials, OAuth,
};
use std::thread;
use tiny_http::{Server, Response};
use url::form_urlencoded;

#[tokio::main]
async fn main() {
    // Initialize the logger (optional)
    env_logger::init();

    // Read credentials and OAuth settings from environment variables
    let creds = Credentials::from_env().expect("Missing client ID or secret in env");
    let oauth = OAuth::from_env(scopes!("user-read-currently-playing")).expect("Missing redirect URI or scopes");

    // Create the Spotify client
    let mut spotify = AuthCodeSpotify::new(creds, oauth.clone());

    // Generate the authorization URL
    let auth_url = spotify.get_authorize_url(false).expect("Failed to get authorize URL");

    // Start a local HTTP server to receive the OAuth callback
    let redirect_port = oauth.redirect_uri.parse::<url::Url>()
        .expect("Invalid redirect URI").port().unwrap_or(8888);
    let server_addr = format!("127.0.0.1:{}", redirect_port);
    let server = Server::http(&server_addr).expect("Failed to start HTTP server");

    // Open the authorization URL in the default browser
    open::that(&auth_url).expect("Failed to open browser");
    println!("Waiting for Spotify authorization callback...");

    // Handle a single incoming request
    if let Ok(request) = server.recv() {
        // Extract the code query parameter
        let query = request.url().split('?').nth(1).unwrap_or("");
        let code_opt = form_urlencoded::parse(query.as_bytes())
            .find(|(k, _)| k == "code")
            .map(|(_, v)| v.into_owned());

        // Respond to the browser
        let response = Response::from_string("<h1>Authorization complete</h1><p>You can close this window.</p>");
        let _ = request.respond(response);

        if let Some(code) = code_opt {
            // Exchange code for access token
            spotify.request_token(&code).await.expect("Failed to request token");
        } else {
            eprintln!("No code found in callback URL.");
            return;
        }
    }

    // Query the currently playing track
    let market = Market::Country(Country::Spain);
    let additional_types = [AdditionalType::Episode];
    let response = spotify
        .current_playing(Some(market), Some(&additional_types))
        .await
        .expect("API request failed");

    // Print human-readable output
    match response {
        Some(context) if context.is_playing => {
            if let Some(item) = context.item {
                match item {
                    rspotify::model::PlayableItem::Track(track) => {
                        let title = &track.name;
                        let artists: Vec<String> = track.artists.iter().map(|a| a.name.clone()).collect();
                        let url = track
                            .external_urls
                            .get("spotify")
                            .map(|s| s.as_str())
                            .unwrap_or("");
                        println!("Now playing: {} — {}", artists.join(", "), title);
                        println!("Listen here: {}", url);
                    }
                    rspotify::model::PlayableItem::Episode(episode) => {
                        let url = episode
                            .external_urls
                            .get("spotify")
                            .map(|s| s.as_str())
                            .unwrap_or("");
                        println!("Now playing episode: {} — {}", episode.name, episode.show.name);
                        println!("Listen here: {}", url);
                    }
                }
            } else {
                println!("No track or episode found in the response.");
            }
        }
        _ => println!("Nothing is currently playing."),
    }
}
