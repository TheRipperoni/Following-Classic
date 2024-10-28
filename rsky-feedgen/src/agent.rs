use crate::models::Follow;
use bsky_sdk::api::com::atproto::repo::list_records::Record;
use bsky_sdk::api::types::string::{AtIdentifier, Nsid};
use bsky_sdk::api::types::Unknown;
use bsky_sdk::BskyAgent;
use ipld_core::ipld::Ipld;
use std::str::FromStr;

#[tracing::instrument(skip(agent))]
pub async fn get_follows(agent: &BskyAgent, did: &str) -> Vec<Follow> {
    use bsky_sdk::api::com::atproto::repo::list_records::{Parameters, ParametersData};
    let mut records: Vec<Record> = Vec::new();
    let mut follows = Vec::new();
    let mut cursor: Option<String> = None;

    match agent
        .api
        .com
        .atproto
        .repo
        .list_records(Parameters {
            data: ParametersData {
                collection: Nsid::new(String::from("app.bsky.graph.follow")).unwrap(),
                cursor,
                limit: None,
                repo: AtIdentifier::from_str(did).unwrap(),
                reverse: None,
                rkey_end: None,
                rkey_start: None,
            },
            extra_data: Ipld::Null,
        })
        .await
    {
        Ok(res) => {
            cursor = res.cursor.clone();
            records = res.records.clone();
        }
        Err(e) => {
            tracing::error!(
                "{}",
                format!(
                    "Error calling get following records: {x}",
                    x = e.to_string()
                )
            );
            cursor = None;
        }
    }
    while cursor.is_some() {
        match agent
            .api
            .com
            .atproto
            .repo
            .list_records(Parameters {
                data: ParametersData {
                    collection: Nsid::new(String::from("app.bsky.graph.follow")).unwrap(),
                    cursor,
                    limit: None,
                    repo: AtIdentifier::from_str(did).unwrap(),
                    reverse: None,
                    rkey_end: None,
                    rkey_start: None,
                },
                extra_data: Ipld::Null,
            })
            .await
        {
            Ok(mut res) => {
                cursor = res.cursor.clone();
                records.append(&mut res.records);
            }
            Err(e) => {
                tracing::error!(
                    "{}",
                    format!(
                        "Error calling get following records: {x}",
                        x = e.to_string()
                    )
                );
                cursor = None;
            }
        }
    }
    for record in records.iter_mut() {
        match record.value.clone() {
            Unknown::Object(obj) => {
                let obj_type = obj.get("$type");
                match obj_type {
                    None => {}
                    Some(x) => {
                        let follow_field: String = <Ipld as Clone>::clone(&x)
                            .try_into()
                            .unwrap_or(String::from("no"));
                        if follow_field == "app.bsky.graph.follow" {
                            let subject: String;
                            let created_at: String;
                            match obj.get("subject") {
                                None => {
                                    panic!()
                                }
                                Some(x) => {
                                    subject = <Ipld as Clone>::clone(&x).try_into().unwrap();
                                }
                            }
                            match obj.get("createdAt") {
                                None => {
                                    panic!()
                                }
                                Some(x) => {
                                    created_at = <Ipld as Clone>::clone(&x).try_into().unwrap();
                                }
                            }

                            let new_follow = Follow {
                                uri: record.uri.clone(),
                                cid: "record.cid.clone()".to_string(),
                                author: did.to_string(),
                                subject,
                                created_at: created_at.clone(),
                                indexed_at: created_at,
                                prev: None,
                                sequence: None,
                            };

                            follows.push(new_follow);
                        }
                    }
                }
            }
            Unknown::Null => {}
            Unknown::Other(_) => {}
        }
    }
    follows
}

pub async fn get_agent() -> Result<BskyAgent, bool> {
    let agent: BskyAgent = BskyAgent::builder().build().await.unwrap();
    Ok(agent)
}
