mod spotify_auth;

#[tokio::main]
async fn main() {
    spotify_auth::spotify_authorize_and_get_current_playing().await;
}