use reqwest::Response;
use rspotify::http::HttpError;
use rspotify::ClientError;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum AuthErrorType {
    #[serde(rename = "invalid_request")]
    InvalidRequest,
    #[serde(rename = "invalid_client")]
    InvalidClient,
    #[serde(rename = "invalid_grant")]
    InvalidGrant,
    #[serde(rename = "unauthorized_client")]
    UnauthorizedClient,
    #[serde(rename = "unsupported_grant_type")]
    UnsupportedGrantType,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthError {
    pub error: AuthErrorType,
    pub error_description: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RegularError {
    pub error: RegularErrorContent,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RegularErrorContent {
    pub status: u32,
    pub message: String,
}

#[derive(From, Debug)]
pub enum Error {
    Auth(AuthError),
    Regular(RegularError),
}

impl Error {
    pub fn extract_response(err: &mut anyhow::Error) -> Option<&mut reqwest::Response> {
        let err = err.downcast_mut::<rspotify::ClientError>()?;

        let ClientError::Http(box HttpError::StatusCode(response)) = err else {
            return None;
        };

        Some(response)
    }

    pub async fn from_anyhow(err: &mut anyhow::Error) -> anyhow::Result<Option<Error>> {
        let Some(response) = Self::extract_response(err) else {
            return Ok(None);
        };

        Self::from_response(response).await.map(Some)
    }

    pub async fn from_response(response: &mut Response) -> anyhow::Result<Error> {
        let body = {
            let mut bytes = vec![];
            while let Some(chunk) = response.chunk().await? {
                bytes.extend(chunk);
            }
            bytes
        };

        let auth = serde_json::from_slice::<AuthError>(&body);
        let regular = serde_json::from_slice::<RegularError>(&body);

        match (auth, regular) {
            (Ok(err), _) => Ok(err.into()),
            (_, Ok(err)) => Ok(err.into()),
            (Err(err), _) => Err(err.into()),
        }
    }
}
