use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use anyhow::{bail, Context};
use reqwest::{
    blocking::{
        multipart::{Form, Part},
        Client, Response,
    },
    header::{HeaderMap, HeaderName, HeaderValue},
    StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The amount of time polling for a status update on an operation can take
/// in seconds.
const POLL_TIMEOUT: u64 = 5 * 60;
/// The maximum delay between poll requests on operations in seconds.
const MAX_POLL_DELAY: u64 = 30;
/// The header that Roblox uses to indicate how long until a rate limit resets.
const RATE_LIMIT_RESET_HEADER: &str = "X-Ratelimit-Reset";

const ASSET_BASE_URL_OPEN_CLOUD: &str = "https://apis.roblox.com/assets/v1";
const ASSET_BASE_URL_COOKIE: &str = "https://apis.roblox.com/assets/user-auth/v1";

/// The header Roblox uses for their API key
const API_KEY_HEADER: &str = "X-API-Key";
/// The header used by Roblox for their CSRF tokens
const CSRF_HEADER: &str = "X-CSRF-Token";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PatchAssetRequestPart {
    asset_id: String,
    published: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PatchAssetResponse {
    // done: bool,
    path: String,
    // operation_id: String,
    error: Option<ResponseError>,
    response: Option<AssetResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResponseError {
    // code: String,
    message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssetResponse {
    // asset_id: String,
    asset_type: String,
    // description: String,
    display_name: String,
    revision_id: String,
    // revision_create_time: String,
}

/// Replaces the asset at `asset_id` with the provided `contents`, authenticating with
/// an Open Cloud API key. Returns the new revision ID of the uploaded asset.
///
/// If `publish` is true, then the asset will be published if it is a place.
#[inline]
pub fn patch_asset_open_cloud(
    api_key: &str,
    asset_id: u64,
    contents: Vec<u8>,
    publish: bool,
) -> anyhow::Result<String> {
    patch_asset(
        (API_KEY_HEADER, api_key),
        ASSET_BASE_URL_OPEN_CLOUD,
        asset_id,
        contents,
        publish,
    )
}

/// Replaces the asset at `asset_id` with the provided `contents`, authenticating with
/// the provided cookie. Returns the new revision ID of the uploaded asset.
///
/// If `publish` is true, then the asset will be published if it is a place.
#[inline]
pub fn patch_asset_cookie(
    cookie: &str,
    asset_id: u64,
    contents: Vec<u8>,
    publish: bool,
) -> anyhow::Result<String> {
    patch_asset(
        ("Cookie", &format!(".ROBLOSECURITY={cookie}")),
        ASSET_BASE_URL_COOKIE,
        asset_id,
        contents,
        publish,
    )
}

/// The meat of the patching logic. This function:
/// - Checks with Roblox to get the asset type of `asset_id` (so we can set the mime type correctly)
/// - Sends a PATCH request to the right URL
/// - If it fails with a 403, it retries to get a CSRF token in case it's needed
/// - If either of the requests succeed, polls for the resulting operation to be finished
/// - Reports either error or success depending upon the result of that poll
/// - Returns the new revision ID if the upload succeeded
///
/// `authentication` is expected to be in the format (Header_Name, Header_Value).
/// `base_url` is meant to be a URL that can be used as a prefix for operations.
/// `asset_id` is obviously the asset ID.
/// `contents` is the contents of the file to upload.
/// `publish` is used to indicate whether a place file should be uploaded.
fn patch_asset(
    authentication: (&str, &str),
    base_url: &str,
    asset_id: u64,
    contents: Vec<u8>,
    publish: bool,
) -> anyhow::Result<String> {
    let client = client();
    let asset_url = format!("{base_url}/assets/{asset_id}");

    let mime_type = match get_asset_type(&client, authentication, asset_id, &asset_url)?.as_str() {
        "Place" => "model/x-rbxl",
        "Model" => "model/x-rbxm",
        "Animation" => "model/x-rbxm",
        other => {
            log::warn!("Unknown asset type '{other}' (please report this to Rojo's developers!). Attempting to upload anyway...");
            "model/x-rbxm"
        }
    };

    let request_fmt = serde_json::to_string(&PatchAssetRequestPart {
        asset_id: asset_id.to_string(),
        published: publish,
    })?;

    let mut headers = HeaderMap::with_capacity(1);
    headers.insert(
        HeaderName::from_bytes(authentication.0.as_bytes())
            .context("provided authentication must have a valid header name")?,
        HeaderValue::from_str(authentication.1)
            .context("provided authentication must have a valid header contents")?,
    );

    // TODO: Is it worth making `contents` into an `Rc<Vec<u8>>` to avoid copying?
    let make_form = || -> anyhow::Result<Form> {
        Ok(Form::new().text("request", request_fmt.clone()).part(
            "fileContent",
            Part::bytes(contents.clone())
                .mime_str(mime_type)?
                // Requests do not work correctly unless the file name is set on
                // this part.
                .file_name("rojo-upload.rbxl"),
        ))
    };

    log::debug!("Sending PATCH request to {asset_url}");
    let mut raw_response = client
        .patch(&asset_url)
        .multipart(make_form()?)
        .header("User-Agent", super::USER_AGENT)
        .headers(headers.clone())
        .send()
        .with_context(|| format!("Request to upload asset {asset_id} failed"))?;

    // To support uploading with a cookie, we have to have a retry for the CSRF token
    // The simplest option is to run it again.
    if raw_response.status() == StatusCode::FORBIDDEN {
        if let Some(csrf_token) = raw_response.headers().get(CSRF_HEADER) {
            log::debug!("Recieved CSRF challenge, retrying with token...");
            headers.insert(
                HeaderName::from_bytes(CSRF_HEADER.as_bytes())
                    .expect("CSRF header is a valid header name"),
                csrf_token.to_owned(),
            );
            raw_response = client
                .patch(&asset_url)
                .multipart(make_form()?)
                .header("User-Agent", super::USER_AGENT)
                .headers(headers.clone())
                .send()
                .with_context(|| format!("Request to upload asset {asset_id} failed"))?;
        }
    }

    match raw_response.status() {
        StatusCode::OK => {}
        StatusCode::TOO_MANY_REQUESTS => {
            let rate_limit_reset = get_ratelimit_reset_header(&raw_response)?;
            bail!(
                "Rate limited by Roblox. Try again in {:.2} seconds",
                rate_limit_reset.as_secs_f64()
            );
        }
        StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED => {
            bail!("Upload failed due to a permission error with your authentication.\nUploading with open cloud requires the scopes `assets:read` and `assets:write`. Double check your API key has those.\nIf you are uploading with a cookie, make sure it has not been invalidated.")
        }
        status => {
            let body = raw_response.text().context(
                "The request to upload to Roblox was malformed (could not be read as text)",
            )?;
            bail!("The request to upload to Roblox failed with status {status}:\n{body}")
        }
    }

    let initial_response: PatchAssetResponse = raw_response.json().context(
        "The response from Roblox for uploading could not be deserialized or was not what was expected",
    )?;

    log::trace!("PATCH request finished");
    log::debug!("{:?}", initial_response);

    // Once the request is made, we need to wait until it's done.
    log::debug!("Polling operation until marked as 'done'");
    let poll_url = format!("{base_url}/{}", initial_response.path);
    let polled: PatchAssetResponse =
        serde_json::from_value(poll_operation(&client, &poll_url, headers)?)
            .context("The response from Roblox for finishing the upload could not be deserialized or was not what was expected")?;
    log::debug!("Polling operation done");

    if let Some(error) = polled.error {
        bail!(
            "The upload completed but Roblox indicates a failure of some kind:\n{}",
            error.message
        );
    }
    if let Some(response) = polled.response {
        let revision = &response.revision_id;
        let name = &response.display_name;
        log::info!("Upload to Roblox completed");
        if publish {
            log::info!("Revision {revision} of asset '{name}' is available and published");
        } else {
            log::info!("Revision {revision} of asset '{name}' is available but not published");
        }
        Ok(response.revision_id)
    } else {
        bail!("The upload completed but did not return an appropriate response")
    }
}

/// If given an API key and an operation ID, will poll it until it is done.
/// Uses exponential backoff and respects rate limits.
///
/// This function will error if the operation takes too long or if an invalid
/// status code is returned.
fn poll_operation(client: &Client, url: &str, headers: HeaderMap) -> anyhow::Result<Value> {
    let now = Instant::now();
    let mut delay = Duration::from_micros(500);

    log::debug!("Polling status of operation from {url}");
    while now.elapsed() <= Duration::from_secs(POLL_TIMEOUT) {
        log::trace!("Polling status of operation...");
        let result = client
            .get(url)
            .header("User-Agent", super::USER_AGENT)
            // We accept a headers map to support both authentication and a CSRF token if needed
            .headers(headers.clone())
            .send()
            .context("Sending a request to poll for the result of the upload failed")?;

        let rate_limit_reset = get_ratelimit_reset_header(&result)?;

        match result.status() {
            StatusCode::TOO_MANY_REQUESTS => {
                log::warn!(
                    "Rate limited by Roblox. Waiting {:.2} seconds to try again...",
                    rate_limit_reset.as_secs_f64()
                );
                sleep(rate_limit_reset);
                delay = Duration::from_micros(500);
                continue;
            }
            StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED => {
                bail!("Upload failed due to a permission error with your authentication.\nUploading with open cloud requires the scopes `assets:read` and `assets:write`. Double check your API key has those.\nIf you are uploading with a cookie, make sure it has not been invalidated.")
            }
            StatusCode::OK => {
                let response: Value = result
                    .json()
                    .context("The response from Roblox for polling the upload status could not be deserialized or was not what was expected")?;
                if let Value::Object(value) = &response {
                    if let Some(done) = value.get("done") {
                        if let Value::Bool(done) = done {
                            if *done {
                                return Ok(response);
                            }
                        }
                    } else {
                        bail!("The response from Roblox for polling the upload status was malformed (missing 'done' field)")
                    }
                } else {
                    bail!("The response from Roblox for polling the upload status was malformed (response was not an object)")
                }
            }
            status => {
                let body = result.text().context(
                    "The request to poll the upload status from Roblox was malformed (could not be read as text)",
                )?;
                bail!("The request to poll the upload status from Roblox failed with status {status}:\n{body}")
            }
        }

        sleep(delay);
        delay = (delay * 2).min(Duration::from_secs(MAX_POLL_DELAY))
    }

    bail!(
        "Polling the upload status took too long. Elapsed time: {:.2}s",
        now.elapsed().as_secs_f64()
    );
}

/// Retrieves the internal AssetType of an asset from Roblox.
fn get_asset_type(
    client: &Client,
    authentication: (&str, &str),
    asset_id: u64,
    url: &str,
) -> anyhow::Result<String> {
    log::debug!("Fetching asset type for {asset_id}");
    let raw_response = client
        .get(url)
        .header(authentication.0, authentication.1)
        .header("User-Agent", super::USER_AGENT)
        .send()
        .with_context(|| format!("The request to get the asset type of asset {asset_id} failed"))?;

    match raw_response.status() {
        StatusCode::OK => {}
        StatusCode::TOO_MANY_REQUESTS => {
            let rate_limit_reset = get_ratelimit_reset_header(&raw_response)?;
            bail!(
                "Rate limited by Roblox. Try again in {:.2} seconds",
                rate_limit_reset.as_secs_f64()
            );
        }
        StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED => {
            bail!("Upload failed due to a permission error with your authentication.\nUploading with open cloud requires the scopes `assets:read` and `assets:write`. Double check your API key has those.\nIf you are uploading with a cookie, make sure it has not been invalidated.")
        }
        status => {
            let body = raw_response.text().context(
                "The request to upload to Roblox was malformed (could not be read as text)",
            )?;
            bail!("The request to upload to Roblox failed with status {status}:\n{body}")
        }
    }

    let response: AssetResponse = raw_response
        .json()
        .context("The response from Roblox for getting the asset type could not be deserialized or was not what was expected")?;

    Ok(response.asset_type)
}

/// When provided a `Response` from Roblox, returns the time until rate limits
/// reset as a `Duration`.
///
/// Errors if the header is missing or if there's an error parsing it.
fn get_ratelimit_reset_header(response: &Response) -> anyhow::Result<Duration> {
    response
        .headers()
        .get(RATE_LIMIT_RESET_HEADER)
        // If Roblox decides to suddenly change how they're throwing
        // rate limits at us, so be it.
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse().ok())
        .map(Duration::from_secs)
        .ok_or_else(|| {
            anyhow::anyhow!("A response from Roblox was malformed (missing rate limit header)")
        })
}

/// Constructs a new Reqwest client. Used instead of directly invoking
/// `Client::new` because it can be desirable to set up a proxy during
/// development.
fn client() -> Client {
    Client::new()
    // reqwest::blocking::ClientBuilder::new()
    //     .proxy(reqwest::Proxy::all("http://localhost:8080").unwrap())
    //     .build()
    //     .unwrap()
}

#[cfg(test)]
mod test {
    use rbx_dom_weak::{InstanceBuilder, WeakDom};
    use uuid::Uuid;

    use super::*;

    // These assets are owned by the Roblox community 'Rojo Foundation'
    // Located here: https://www.roblox.com/communities/32644114/Rojo-Foundation
    // To avoid accidentally double-uploading an asset (and thus ruining the test)
    // we use a separate place/model for each upload type.
    const OPEN_CLOUD_MODEL_ID: u64 = 136940743748740;
    const COOKIE_MODEL_ID: u64 = 136033424984300;
    const OPEN_CLOUD_PLACE_ID: u64 = 73509996833283;
    const COOKIE_PLACE_ID: u64 = 99207739640685;

    #[test]
    fn upload_model_open_cloud() {
        if !matches!(std::env::var("CI").as_deref(), Ok("true")) {
            println!("upload tests only run in CI in Rojo's repository due to requiring secrets");
            return;
        }
        let Ok(api_key) = std::env::var("UPLOAD_COMMAND_TEST_TOKEN") else {
            panic!("no 'UPLOAD_COMMAND_TEST_TOKEN' env var set")
        };

        let old_revision_response: serde_json::Value = client()
            .get(format!(
                "{ASSET_BASE_URL_OPEN_CLOUD}/assets/{OPEN_CLOUD_MODEL_ID}"
            ))
            .header("User-Agent", crate::web::USER_AGENT)
            .header(API_KEY_HEADER, &api_key)
            .send()
            .expect("requests to Roblox should not fail")
            .json()
            .expect("deserializing the response from Roblox as JSON should not fail");

        println!("response for old revision: {}", old_revision_response);

        let old_revision = serde_json::from_value::<AssetResponse>(old_revision_response)
            .expect("deserializing the response from Roblox should not fail")
            .revision_id;

        let dom = WeakDom::new(InstanceBuilder::new("Workspace").with_name(Uuid::new_v4()));

        let mut buffer = Vec::new();
        rbx_binary::to_writer(&mut buffer, &dom, &[dom.root_ref()]).unwrap();

        let new_revision =
            patch_asset_open_cloud(&api_key, OPEN_CLOUD_MODEL_ID, buffer, true).unwrap();

        assert_ne!(
            old_revision, new_revision,
            "revision ID before uploading should not equal revision ID after uploading"
        )
    }

    #[test]
    fn upload_place_open_cloud() {
        if !matches!(std::env::var("CI").as_deref(), Ok("true")) {
            println!("upload tests only run in CI in Rojo's repository due to requiring secrets");
            return;
        }
        let Ok(api_key) = std::env::var("UPLOAD_COMMAND_TEST_TOKEN") else {
            panic!("no 'UPLOAD_COMMAND_TEST_TOKEN' env var set")
        };

        let old_revision_response: serde_json::Value = client()
            .get(format!(
                "{ASSET_BASE_URL_OPEN_CLOUD}/assets/{OPEN_CLOUD_PLACE_ID}"
            ))
            .header("User-Agent", crate::web::USER_AGENT)
            .header(API_KEY_HEADER, &api_key)
            .send()
            .expect("requests to Roblox should not fail")
            .json()
            .expect("deserializing the response from Roblox as JSON should not fail");

        println!("response for old revision: {}", old_revision_response);

        let old_revision = serde_json::from_value::<AssetResponse>(old_revision_response)
            .expect("deserializing the response from Roblox should not fail")
            .revision_id;

        let dom = WeakDom::new(InstanceBuilder::new("Workspace").with_name(Uuid::new_v4()));

        let mut buffer = Vec::new();
        rbx_binary::to_writer(&mut buffer, &dom, &[dom.root_ref()]).unwrap();

        let new_revision =
            patch_asset_open_cloud(&api_key, OPEN_CLOUD_PLACE_ID, buffer, true).unwrap();

        assert_ne!(
            old_revision, new_revision,
            "revision ID before uploading should not equal revision ID after uploading"
        )
    }

    #[test]
    fn upload_model_cookie() {
        if !matches!(std::env::var("CI").as_deref(), Ok("true")) {
            println!("upload tests only run in CI in Rojo's repository due to requiring secrets");
            return;
        }
        let Ok(api_key) = std::env::var("UPLOAD_COMMAND_TEST_TOKEN") else {
            panic!("no 'UPLOAD_COMMAND_TEST_TOKEN' env var set")
        };
        let Ok(cookie) = std::env::var("BOT_ACCOUNT_COOKIE") else {
            panic!("no 'BOT_ACCOUNT_COOKIE' env var set")
        };

        let old_revision_response: serde_json::Value = client()
            .get(format!(
                "{ASSET_BASE_URL_OPEN_CLOUD}/assets/{COOKIE_MODEL_ID}"
            ))
            .header("User-Agent", crate::web::USER_AGENT)
            .header(API_KEY_HEADER, &api_key)
            .send()
            .expect("requests to Roblox should not fail")
            .json()
            .expect("deserializing the response from Roblox as JSON should not fail");

        println!("response for old revision: {}", old_revision_response);

        let old_revision = serde_json::from_value::<AssetResponse>(old_revision_response)
            .expect("deserializing the response from Roblox should not fail")
            .revision_id;

        let dom = WeakDom::new(InstanceBuilder::new("Workspace").with_name(Uuid::new_v4()));

        let mut buffer = Vec::new();
        rbx_binary::to_writer(&mut buffer, &dom, &[dom.root_ref()]).unwrap();

        let new_revision = patch_asset_cookie(&cookie, COOKIE_MODEL_ID, buffer, true).unwrap();

        assert_ne!(
            old_revision, new_revision,
            "revision ID before uploading should not equal revision ID after uploading"
        )
    }

    #[test]
    fn upload_place_cookie() {
        if !matches!(std::env::var("CI").as_deref(), Ok("true")) {
            println!("upload tests only run in CI in Rojo's repository due to requiring secrets");
            return;
        }
        let Ok(api_key) = std::env::var("UPLOAD_COMMAND_TEST_TOKEN") else {
            panic!("no 'UPLOAD_COMMAND_TEST_TOKEN' env var set")
        };
        let Ok(cookie) = std::env::var("BOT_ACCOUNT_COOKIE") else {
            panic!("no 'BOT_ACCOUNT_COOKIE' env var set")
        };

        let old_revision_response: serde_json::Value = client()
            .get(format!(
                "{ASSET_BASE_URL_OPEN_CLOUD}/assets/{COOKIE_PLACE_ID}"
            ))
            .header("User-Agent", crate::web::USER_AGENT)
            .header(API_KEY_HEADER, &api_key)
            .send()
            .expect("requests to Roblox should not fail")
            .json()
            .expect("deserializing the response from Roblox as JSON should not fail");

        println!("response for old revision: {}", old_revision_response);

        let old_revision = serde_json::from_value::<AssetResponse>(old_revision_response)
            .expect("deserializing the response from Roblox should not fail")
            .revision_id;

        let dom = WeakDom::new(InstanceBuilder::new("Workspace").with_name(Uuid::new_v4()));

        let mut buffer = Vec::new();
        rbx_binary::to_writer(&mut buffer, &dom, &[dom.root_ref()]).unwrap();

        let new_revision = patch_asset_cookie(&cookie, COOKIE_PLACE_ID, buffer, true).unwrap();

        assert_ne!(
            old_revision, new_revision,
            "revision ID before uploading should not equal revision ID after uploading"
        )
    }
}
