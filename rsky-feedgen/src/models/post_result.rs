#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct PostResult {
    #[serde(rename = "post")]
    pub post: String,
    #[serde(rename = "reason", skip_serializing_if = "Option::is_none")]
    pub reason: Option<PostResultReason>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct PostResultReason {
    #[serde(rename = "$type")]
    pub reason_type: String,
    #[serde(rename = "repost")]
    pub repost_uri: String,
}
