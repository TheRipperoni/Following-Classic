#[macro_use]
extern crate rocket;
use dotenvy::dotenv;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::figment::{
    util::map,
    value::{Map, Value},
};
use rocket::http::Header;
use rocket::http::Status;
use rocket::response::status;
use rocket::serde::json::Json;
use rocket::{Request, Response};
use rsky_feedgen::models::{AlgoResponse, FollowingPreference, JwtParts, PostResult, UserFeedPreference};
use rsky_feedgen::{ReadReplicaConn, WriteDbConn};
use std::env;

pub struct CORS;

use rocket::request::{FromRequest, Outcome};
use serde_derive::{Deserialize, Serialize};

#[allow(dead_code)]
struct ApiKey<'r>(&'r str);

#[derive(Debug)]
struct AccessToken(String);

#[derive(Debug)]
enum ApiKeyError {
    Missing,
    Invalid,
}

#[derive(Debug)]
enum AccessTokenError {
    Missing,
    Invalid,
}

#[allow(unused_assignments)]
#[rocket::async_trait]
impl<'r> FromRequest<'r> for ApiKey<'r> {
    type Error = ApiKeyError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let mut token: String = "".to_owned();
        if let Ok(token_result) = env::var("RSKY_API_KEY") {
            token = token_result;
        } else {
            return Outcome::Error((Status::BadRequest, ApiKeyError::Invalid));
        }

        match req.headers().get_one("X-RSKY-KEY") {
            None => Outcome::Error((Status::Unauthorized, ApiKeyError::Missing)),
            Some(key) if key == token => Outcome::Success(ApiKey(key)),
            Some(_) => Outcome::Error((Status::Unauthorized, ApiKeyError::Invalid)),
        }
    }
}

#[allow(unused_assignments)]
#[rocket::async_trait]
impl<'r> FromRequest<'r> for AccessToken {
    type Error = AccessTokenError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match req.headers().get_one("Authorization") {
            None => Outcome::Error((Status::Unauthorized, AccessTokenError::Missing)),
            Some(token) if !token.starts_with("Bearer ") => {
                Outcome::Error((Status::Unauthorized, AccessTokenError::Invalid))
            }
            Some(token) => {
                println!("Visited by {token:?}");
                let service_did = env::var("FEEDGEN_SERVICE_DID").unwrap_or("".into());
                let jwt = token.split(" ").map(String::from).collect::<Vec<_>>();
                if let Some(jwtstr) = jwt.last() {
                    match rsky_feedgen::auth::verify_jwt(&jwtstr, &service_did) {
                        Ok(jwt_object) => Outcome::Success(AccessToken(jwt_object)),
                        Err(error) => {
                            eprintln!("Error decoding jwt. {error:?}");
                            Outcome::Error((Status::Unauthorized, AccessTokenError::Invalid))
                        }
                    }
                } else {
                    Outcome::Error((Status::Unauthorized, AccessTokenError::Invalid))
                }
            }
        }
    }
}

const FOLLOWING_TRAD: &str =
    "at://did:plc:khvyd3oiw46vif5gm7hijslk/app.bsky.feed.generator/following-trad";
const FOLLOWING_CLASSIC: &str =
    "at://did:plc:cimwguwdlh2i2mebdqczgcyl/app.bsky.feed.generator/follow-orig";

#[get(
    "/xrpc/app.bsky.feed.getFeedSkeleton?<feed>&<limit>&<cursor>",
    format = "json"
)]
async fn index(
    feed: Option<&str>,
    limit: Option<i64>,
    cursor: Option<&str>,
    connection: ReadReplicaConn,
    _token: Result<AccessToken, AccessTokenError>,
) -> Result<
    Json<rsky_feedgen::models::AlgoResponse>,
    status::Custom<Json<rsky_feedgen::models::InternalErrorMessageResponse>>,
> {
    let mut did = String::from("did:plc:khvyd3oiw46vif5gm7hijslk");
    let feed = feed.unwrap_or("".into());
    if let Ok(jwt) = _token {
        match serde_json::from_str::<JwtParts>(&jwt.0) {
            Ok(jwt_obj) => {
                did = jwt_obj.iss;
                match rsky_feedgen::apis::add_visitor(did.clone(), jwt_obj.aud, feed.to_string()) {
                    Ok(_) => (),
                    Err(_) => eprintln!("Failed to write visitor."),
                }
            }
            Err(_) => eprintln!("Failed to parse jwt string."),
        }
    } else {
        let service_did = env::var("FEEDGEN_SERVICE_DID").unwrap_or("".into());
        match rsky_feedgen::apis::add_visitor("anonymous".into(), service_did, feed.to_string()) {
            Ok(_) => (),
            Err(_) => eprintln!("Failed to write anonymous visitor."),
        }
    }
    match feed {
        _following_classic if FOLLOWING_CLASSIC == _following_classic => {
            if did == "" {
                let internal_error = rsky_feedgen::models::InternalErrorMessageResponse {
                    code: Some(rsky_feedgen::models::InternalErrorCode::InternalError),
                    message: Some("No DID".to_string()),
                };
                return Err(status::Custom(
                    Status::InternalServerError,
                    Json(internal_error),
                ))
            }
            match rsky_feedgen::apis::get_posts_by_user_feed(did, limit, cursor, connection)
            .await
            {
                Ok(response) => Ok(Json(response)),
                Err(error) => {
                    eprintln!("Internal Error: {error}");
                    let internal_error = rsky_feedgen::models::InternalErrorMessageResponse {
                        code: Some(rsky_feedgen::models::InternalErrorCode::InternalError),
                        message: Some(error.to_string()),
                    };
                    Err(status::Custom(
                        Status::InternalServerError,
                        Json(internal_error),
                    ))
                }
            }
        }
        _following_trad if FOLLOWING_TRAD == _following_trad => {
            let mut post_results = Vec::new();
            let post_result = PostResult { post: String::from("at://did:plc:cimwguwdlh2i2mebdqczgcyl/app.bsky.feed.post/3l4pi6irzsg2m"), reason: None };
            post_results.push(post_result);
            let response = AlgoResponse {
                cursor: Some(String::from("none")),
                feed: post_results,
            };
            Ok(Json(response))
        }
        _ => {
            let internal_error = rsky_feedgen::models::InternalErrorMessageResponse {
                code: Some(rsky_feedgen::models::InternalErrorCode::InternalError),
                message: Some("Not Found".to_string()),
            };
            Err(status::Custom(
                Status::InternalServerError,
                Json(internal_error),
            ))
        }
    }
}

#[put("/cursor?<service>&<sequence>")]
async fn update_cursor(
    service: &str,
    sequence: i64,
    _key: ApiKey<'_>,
    connection: WriteDbConn,
) -> Result<(), status::Custom<Json<rsky_feedgen::models::InternalErrorMessageResponse>>> {
    match rsky_feedgen::apis::update_cursor(service.to_string(), sequence, connection).await {
        Ok(_) => Ok(()),
        Err(error) => {
            eprintln!("Internal Error: {error}");
            let internal_error = rsky_feedgen::models::InternalErrorMessageResponse {
                code: Some(rsky_feedgen::models::InternalErrorCode::InternalError),
                message: Some(error.to_string()),
            };
            Err(status::Custom(
                Status::InternalServerError,
                Json(internal_error),
            ))
        }
    }
}

#[get("/cursor?<service>", format = "json")]
async fn get_cursor(
    service: &str,
    _key: ApiKey<'_>,
    connection: ReadReplicaConn,
) -> Result<
    Json<rsky_feedgen::models::SubState>,
    status::Custom<Json<rsky_feedgen::models::PathUnknownErrorMessageResponse>>,
> {
    match rsky_feedgen::apis::get_cursor(service.to_string(), connection).await {
        Ok(response) => Ok(Json(response)),
        Err(error) => {
            eprintln!("Internal Error: {error}");
            let path_error = rsky_feedgen::models::PathUnknownErrorMessageResponse {
                code: Some(rsky_feedgen::models::NotFoundErrorCode::NotFoundError),
                message: Some("Not Found".to_string()),
            };
            Err(status::Custom(Status::NotFound, Json(path_error)))
        }
    }
}

#[put("/queue/<lex>/create", format = "json", data = "<body>")]
async fn queue_creation(
    lex: &str,
    body: Json<Vec<rsky_feedgen::models::CreateRequest>>,
    _key: ApiKey<'_>,
    connection: WriteDbConn,
) -> Result<(), status::Custom<Json<rsky_feedgen::models::InternalErrorMessageResponse>>> {
    match rsky_feedgen::apis::queue_creation(lex.to_string(), body.into_inner(), connection).await {
        Ok(_) => Ok(()),
        Err(error) => {
            eprintln!("Internal Error: {error}");
            let internal_error = rsky_feedgen::models::InternalErrorMessageResponse {
                code: Some(rsky_feedgen::models::InternalErrorCode::InternalError),
                message: Some(error.to_string()),
            };
            Err(status::Custom(
                Status::InternalServerError,
                Json(internal_error),
            ))
        }
    }
}

#[get("/user_feed_preference?<did>", format = "json")]
async fn user_config(
    did: &str,
    _key: ApiKey<'_>,
    connection: WriteDbConn,
) -> Result<Json<Vec<UserFeedPreference>>, status::Custom<Json<rsky_feedgen::models::InternalErrorMessageResponse>>> {
    let result = rsky_feedgen::db::user_config_fetch(String::from(did), connection).await;
    Ok(Json::from(result))
}

#[derive(Debug, Serialize, Deserialize)]
struct FollowingPrefFetchResponse {
    pub did: String,
    pub following_preferences: Vec<FollowingPreference>
}

#[get("/following_preferences?<did>", format = "json")]
async fn following_preferences_fetch(
    did: &str,
    connection: WriteDbConn,
) -> Result<Json<FollowingPrefFetchResponse>, status::Custom<Json<rsky_feedgen::models::InternalErrorMessageResponse>>> {
    let result = rsky_feedgen::db::following_pref_fetch(String::from(did), connection).await;
    let response = FollowingPrefFetchResponse {
        did: String::from(did),
        following_preferences: result,
    };
    Ok(Json::from(response))
}

#[put("/following_preferences", format = "json", data = "<body>")]
async fn following_preferences_update(
    body: Json<FollowingPreference>,
    connection: WriteDbConn,
) -> Result<(), status::Custom<Json<rsky_feedgen::models::InternalErrorMessageResponse>>> {
    match rsky_feedgen::db::following_pref_update(body.into_inner(), connection).await {
        Ok(_) => Ok(()),
        Err(error) => {
            eprintln!("Internal Error: {error}");
            let internal_error = rsky_feedgen::models::InternalErrorMessageResponse {
                code: Some(rsky_feedgen::models::InternalErrorCode::InternalError),
                message: Some(error.to_string()),
            };
            Err(status::Custom(
                Status::InternalServerError,
                Json(internal_error),
            ))
        }
    }
}

#[put("/user_feed_preference", format = "json", data = "<body>")]
async fn update_user_config(
    body: Json<UserFeedPreference>,
    _key: ApiKey<'_>,
    connection: WriteDbConn,
) -> Result<(), status::Custom<Json<rsky_feedgen::models::InternalErrorMessageResponse>>> {
    match rsky_feedgen::db::user_config_creation(body.into_inner(), connection).await {
        Ok(_) => Ok(()),
        Err(error) => {
            eprintln!("Internal Error: {error}");
            let internal_error = rsky_feedgen::models::InternalErrorMessageResponse {
                code: Some(rsky_feedgen::models::InternalErrorCode::InternalError),
                message: Some(error.to_string()),
            };
            Err(status::Custom(
                Status::InternalServerError,
                Json(internal_error),
            ))
        }
    }
}

#[put("/queue/<lex>/delete", format = "json", data = "<body>")]
async fn queue_deletion(
    lex: &str,
    body: Json<Vec<rsky_feedgen::models::DeleteRequest>>,
    _key: ApiKey<'_>,
    connection: WriteDbConn,
) -> Result<(), status::Custom<Json<rsky_feedgen::models::InternalErrorMessageResponse>>> {
    match rsky_feedgen::apis::queue_deletion(lex.to_string(), body.into_inner(), connection).await {
        Ok(_) => Ok(()),
        Err(error) => {
            eprintln!("Internal Error: {error}");
            let internal_error = rsky_feedgen::models::InternalErrorMessageResponse {
                code: Some(rsky_feedgen::models::InternalErrorCode::InternalError),
                message: Some(error.to_string()),
            };
            Err(status::Custom(
                Status::InternalServerError,
                Json(internal_error),
            ))
        }
    }
}

#[get("/.well-known/did.json", format = "json")]
async fn well_known() -> Result<
    Json<rsky_feedgen::models::WellKnown>,
    status::Custom<Json<rsky_feedgen::models::PathUnknownErrorMessageResponse>>,
> {
    match env::var("FEEDGEN_SERVICE_DID") {
        Ok(service_did) => {
            let hostname = env::var("FEEDGEN_HOSTNAME").unwrap_or("".into());
            if !service_did.ends_with(hostname.as_str()) {
                let path_error = rsky_feedgen::models::PathUnknownErrorMessageResponse {
                    code: Some(rsky_feedgen::models::NotFoundErrorCode::NotFoundError),
                    message: Some("Not Found".to_string()),
                };
                Err(status::Custom(Status::NotFound, Json(path_error)))
            } else {
                let known_service = rsky_feedgen::models::KnownService {
                    id: "#bsky_fg".to_owned(),
                    r#type: "BskyFeedGenerator".to_owned(),
                    service_endpoint: format!("https://{}", hostname),
                };
                let result = rsky_feedgen::models::WellKnown {
                    context: vec!["https://www.w3.org/ns/did/v1".into()],
                    id: service_did,
                    service: vec![known_service],
                };
                Ok(Json(result))
            }
        }
        Err(_) => {
            let path_error = rsky_feedgen::models::PathUnknownErrorMessageResponse {
                code: Some(rsky_feedgen::models::NotFoundErrorCode::NotFoundError),
                message: Some("Not Found".to_string()),
            };
            Err(status::Custom(Status::NotFound, Json(path_error)))
        }
    }
}

#[catch(404)]
async fn not_found() -> Json<rsky_feedgen::models::PathUnknownErrorMessageResponse> {
    let path_error = rsky_feedgen::models::PathUnknownErrorMessageResponse {
        code: Some(rsky_feedgen::models::NotFoundErrorCode::UndefinedEndpoint),
        message: Some("Not Found".to_string()),
    };
    Json(path_error)
}

#[catch(422)]
async fn unprocessable_entity() -> Json<rsky_feedgen::models::ValidationErrorMessageResponse> {
    let validation_error = rsky_feedgen::models::ValidationErrorMessageResponse {
        code: Some(rsky_feedgen::models::ErrorCode::ValidationError),
        message: Some(
            "The request was well-formed but was unable to be followed due to semantic errors."
                .to_string(),
        ),
    };
    Json(validation_error)
}

#[catch(400)]
async fn bad_request() -> Json<rsky_feedgen::models::ValidationErrorMessageResponse> {
    let validation_error = rsky_feedgen::models::ValidationErrorMessageResponse {
        code: Some(rsky_feedgen::models::ErrorCode::ValidationError),
        message: Some("The request was improperly formed.".to_string()),
    };
    Json(validation_error)
}

#[catch(401)]
async fn unauthorized() -> Json<rsky_feedgen::models::InternalErrorMessageResponse> {
    let internal_error = rsky_feedgen::models::InternalErrorMessageResponse {
        code: Some(rsky_feedgen::models::InternalErrorCode::Unavailable),
        message: Some("Request could not be processed.".to_string()),
    };
    Json(internal_error)
}

#[catch(default)]
async fn default_catcher() -> Json<rsky_feedgen::models::InternalErrorMessageResponse> {
    let internal_error = rsky_feedgen::models::InternalErrorMessageResponse {
        code: Some(rsky_feedgen::models::InternalErrorCode::InternalError),
        message: Some("Internal error.".to_string()),
    };
    Json(internal_error)
}

/// Catches all OPTION requests in order to get the CORS related Fairing triggered.
#[options("/<_..>")]
async fn all_options() {
    /* Intentionally left empty */
}

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "POST, GET, PATCH, OPTIONS, DELETE",
        ));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}

#[launch]
fn rocket() -> _ {
    dotenv().ok();

    let write_database_url = env::var("DATABASE_URL").unwrap_or("".into());
    let read_database_url = env::var("READ_REPLICA_URL").unwrap_or("".into());

    let write_db: Map<_, Value> = map! {
        "url" => write_database_url.into(),
        "pool_size" => 20.into(),
        "timeout" => 30.into(),
    };

    let read_db: Map<_, Value> = map! {
        "url" => read_database_url.into(),
        "pool_size" => 20.into(),
        "timeout" => 30.into(),
    };

    let figment = rocket::Config::figment().merge((
        "databases",
        map!["pg_read_replica" => read_db, "pg_db" => write_db],
    ));

    rocket::custom(figment)
        .mount(
            "/",
            routes![
                index,
                user_config,
                update_user_config,
                queue_creation,
                queue_deletion,
                well_known,
                get_cursor,
                update_cursor,
                all_options,
                following_preferences_fetch,
                following_preferences_update
            ],
        )
        .register(
            "/",
            catchers![
                default_catcher,
                unprocessable_entity,
                bad_request,
                not_found,
                unauthorized
            ],
        )
        .attach(CORS)
        .attach(WriteDbConn::fairing())
        .attach(ReadReplicaConn::fairing())
}
