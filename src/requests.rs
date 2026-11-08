//! Shared JSON request bodies (the Go handlers' anonymous structs).

use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct Credentials {
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
}

#[derive(Deserialize, Default)]
pub struct ComposeRequest {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub compose_content: String,
}

#[derive(Deserialize, Default)]
pub struct CommandRequest {
    #[serde(default)]
    pub command: String,
}

#[derive(Deserialize, Default)]
pub struct ImageRequest {
    #[serde(default)]
    pub image: String,
}

#[derive(Deserialize, Default)]
pub struct NetworkRequest {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub driver: String,
}

#[derive(Deserialize, Default)]
pub struct VolumeRequest {
    #[serde(default)]
    pub name: String,
}
