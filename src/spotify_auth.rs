use rspotify::{
    model::{AdditionalType, Country, Market, PlayableItem},
    prelude::*,
    scopes, AuthCodeSpotify, Credentials, OAuth,
};
use tiny_http::{Server, Response};
use url::form_urlencoded;

pub async fn spotify_authorize_and_get_current_playing() {

    env_logger::try_init().ok();


    let creds = Credentials::from_env().expect("Missing client_id or client_secret");


    let oauth = OAuth::from_env(scopes!("user-read-currently-playing")).expect("Missing redirect URI or scopes");

    let mut spotify = AuthCodeSpotify::new(creds, oauth.clone());


    let auth_url = spotify.get_authorize_url(false).expect("Failed to get authorize URL");


    let redirect_port = oauth.redirect_uri.parse::<url::Url>()
        .expect("Invalid redirect URI").port().unwrap_or(8888);
    let server_addr = format!("127.0.0.1:{}", redirect_port);
    let server = Server::http(&server_addr).expect("Failed to start HTTP server");


    open::that(&auth_url).expect("Failed to open browser");
    println!("Waiting for Spotify authorization callback...");

    if let Ok(request) = server.recv() {

        let query = request.url().split('?').nth(1).unwrap_or("");
        let code_opt = form_urlencoded::parse(query.as_bytes())
            .find(|(k, _)| k == "code")
            .map(|(_, v)| v.into_owned());


        let response = Response::from_string("<h1>Authorization complete</h1><p>You can close this window.</p>");
        let _ = request.respond(response);

        if let Some(code) = code_opt {
            spotify.request_token(&code).await.expect("Failed to request token");
        } else {
            eprintln!("No code found in callback URL.");
            return;
        }
    }

    let market = Market::Country(Country::Spain);
    let additional_types = [AdditionalType::Episode];
    let response = spotify
        .current_playing(Some(market), Some(&additional_types))
        .await
        .expect("API request failed");

    match response {
        Some(context) if context.is_playing => {
            if let Some(item) = context.item {
                match item {
                    PlayableItem::Track(track) => {
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
                    PlayableItem::Episode(episode) => {
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