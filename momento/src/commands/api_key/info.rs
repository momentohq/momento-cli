use super::display::format_epoch_seconds;
use super::permission_protos::{PermissionsProtoV1, PermissionsProtoV2};
use crate::commands::custom_role::utils::Permissions;
use crate::error::CliError;

use base64::{
    engine::{general_purpose::STANDARD, GeneralPurpose},
    Engine,
};
use prost::Message;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const BASE64: GeneralPurpose = GeneralPurpose::new(
    &base64::alphabet::URL_SAFE,
    base64::engine::general_purpose::GeneralPurposeConfig::new()
        .with_decode_padding_mode(base64::engine::DecodePaddingMode::Indifferent),
);

pub(crate) fn get_key_id_from_jwt(auth_token: &str) -> Result<Option<String>, CliError> {
    let header = jsonwebtoken::decode_header(auth_token)
        .map_err(|error| CliError::new(format!("{error}")))?;
    Ok(header.kid)
}

struct Classified {
    kind: &'static str,
    identity_label: &'static str,
    identity_value: String,
    permission_encoding: Option<EmbeddedPermissionEncoding>,
}

fn classify(claims: &Claims, auth_token: &str) -> Result<Classified, CliError> {
    match claims.token_type.as_deref() {
        Some("disposable") => match &claims.version {
            Some(1) => Ok(Classified {
                kind: "Disposable token (v1)",
                identity_label: "Customer",
                identity_value: claims.get_subject_claim()?,
                permission_encoding: Some(EmbeddedPermissionEncoding::V1),
            }),
            Some(2) => Ok(Classified {
                kind: "Disposable token (v2)",
                identity_label: "Account",
                identity_value: claims.get_account_id_claim()?,
                permission_encoding: Some(EmbeddedPermissionEncoding::V2),
            }),
            Some(version) => Err(CliError::new(format!(
                "Unsupported disposable token version {version}"
            ))),
            None => Err(CliError::new("No version for disposable token".to_string())),
        },
        Some("g") => Ok(Classified {
            kind: "Global API key",
            identity_label: "Key",
            identity_value: claims.get_jwt_token_id_claim()?,
            permission_encoding: None,
        }),
        Some("gr") => Ok(Classified {
            kind: "Global API key refresh token",
            identity_label: "Key",
            identity_value: claims.get_api_key_id_claim()?,
            permission_encoding: None,
        }),
        Some(other) => Err(CliError::new(format!("Unknown token type {other}"))),
        None => match &claims.issuer {
            Some(issuer) => Err(CliError::new(format!("Unsupported issuer: {issuer}"))),
            None => {
                if claims.version == Some(1) {
                    Ok(Classified {
                        kind: "API token (v1)",
                        identity_label: "Customer",
                        identity_value: claims.get_subject_claim()?,
                        permission_encoding: Some(EmbeddedPermissionEncoding::V1),
                    })
                } else if claims.control_plane_proxy_endpoint.is_some() {
                    Ok(Classified {
                        kind: "Legacy token",
                        identity_label: "Customer",
                        identity_value: claims.get_subject_claim()?,
                        permission_encoding: None,
                    })
                } else {
                    // The only way to identify a customer signed token is by checking that
                    // a keyid exists in the header, and none of the other claim tags identifying
                    // other token types were found.
                    let key_id =
                        get_key_id_from_jwt(auth_token)?.ok_or(CliError::new("Missing key id"))?;
                    Ok(Classified {
                        kind: "Customer-signed token",
                        identity_label: "Signing key",
                        identity_value: key_id,
                        permission_encoding: None,
                    })
                }
            }
        },
    }
}

/// Claims from all token types, used for initial parsing to determine token type.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    #[serde(rename = "sub")]
    pub subject: Option<String>,

    #[serde(rename = "email")]
    pub email: Option<String>,

    #[serde(rename = "iss")]
    pub issuer: Option<String>,

    #[serde(rename = "ver")]
    pub version: Option<i32>,

    #[serde(rename = "cp")]
    pub control_plane_proxy_endpoint: Option<String>, // control endpoint in Momento Legacy auth tokens

    #[serde(rename = "t")]
    pub token_type: Option<String>, // Momento token type

    #[serde(rename = "jti")]
    pub jwt_token_id: Option<String>, // Token ID on Global Api Key refresh tokens, Key ID on Global Api Keys

    #[serde(rename = "a")]
    pub account_id: Option<String>,

    #[serde(rename = "akid")]
    pub api_key_id: Option<String>, // API key ID on Global Api Key refresh tokens
}

impl Claims {
    fn get_account_id_claim(&self) -> Result<String, CliError> {
        match self.account_id.clone() {
            Some(account_id) => {
                if account_id.is_empty() {
                    Err(CliError::new("Missing account_id claim"))
                } else {
                    Ok(account_id)
                }
            }
            None => Err(CliError::new("Missing account_id claim")),
        }
    }

    fn get_subject_claim(&self) -> Result<String, CliError> {
        match self.subject.clone() {
            Some(subject) => {
                if subject.is_empty() {
                    Err(CliError::new("Missing sub claim"))
                } else {
                    Ok(subject)
                }
            }
            None => Err(CliError::new("Missing sub claim")),
        }
    }

    fn get_jwt_token_id_claim(&self) -> Result<String, CliError> {
        self.jwt_token_id
            .clone()
            .filter(|token_id| !token_id.is_empty())
            .ok_or(CliError::new("Missing jti claim"))
    }

    fn get_api_key_id_claim(&self) -> Result<String, CliError> {
        self.api_key_id
            .clone()
            .filter(|key_id| !key_id.is_empty())
            .ok_or(CliError::new("Missing api key id claim"))
    }
}

/// Initial read of token to determine token type.
pub fn read_buffered_claims(auth_token: &str, buffer: &mut Vec<u8>) -> Result<Claims, CliError> {
    let middle = auth_token
        .split('.')
        .nth(1)
        .ok_or_else(|| CliError::new("JWS schema violated"))?;

    BASE64
        .decode_vec(middle, buffer)
        .map_err(|error| CliError::new(format!("{error}")))?;

    serde_json::de::from_slice(buffer)
        .map_err(|error| CliError::new(format!("JSON issue: {error}")))
}

/// What a token decoded to.
pub struct DecodedApiKey {
    /// What kind of Momento token this is.
    pub kind: &'static str,
    /// Who it identifies — the meaning depends on `kind`, so the label does too.
    pub identity_label: &'static str,
    pub identity_value: String,
    /// The cell endpoint, when the token arrived in an envelope carrying one.
    pub endpoint: Option<String>,
    pub expires: Option<String>,
    pub permissions: Option<Permissions>,
    pub claims: Value,
}

/// Offline and unverified: nothing is contacted and no signature is checked.
/// That is the point — a token that fails validation can still decode here,
/// which is what you need when one is behaving unexpectedly.
pub fn decode(key: &str) -> Result<DecodedApiKey, CliError> {
    let (endpoint, jwt) = unwrap_envelope(key);

    let mut buffer = Vec::new();
    let claims = read_buffered_claims(&jwt, &mut buffer).map_err(|error| {
        CliError::new("This does not read as a Momento token. Pass the key exactly as issued.")
            .with_details(format!("{error}"))
    })?;
    let classified = classify(&claims, &jwt).map_err(|error| {
        CliError::new("Could not determine what kind of token this is")
            .with_details(format!("{error}"))
    })?;

    let payload = serde_json::from_slice::<Value>(&buffer).map_err(|error| {
        CliError::new("the JWT payload is not JSON").with_details(format!("{error}"))
    })?;
    let expires = payload
        .get("exp")
        .and_then(Value::as_u64)
        .map(format_epoch_seconds);
    let permissions = permissions(
        classified.permission_encoding,
        payload.get("p").and_then(Value::as_str),
    )?;

    Ok(DecodedApiKey {
        kind: classified.kind,
        identity_label: classified.identity_label,
        identity_value: classified.identity_value,
        endpoint,
        expires,
        permissions,
        claims: payload,
    })
}

/// Console- and SDK-issued keys arrive as base64 JSON carrying the cell
/// endpoint alongside the JWT; everything else is the JWT itself.
/// Anything that does not read as the envelope is passed through untouched,
/// and fails later as a bad JWT if that is what it is.
fn unwrap_envelope(key: &str) -> (Option<String>, String) {
    #[derive(Deserialize)]
    struct Envelope {
        api_key: String,
        endpoint: String,
    }
    match STANDARD
        .decode(key)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Envelope>(&bytes).ok())
    {
        Some(envelope) => (Some(envelope.endpoint), envelope.api_key),
        None => (None, key.to_owned()),
    }
}

/// The `p` claim: a base64 protobuf permission set, on the token kinds that carry one.
/// Present but empty means super-user — that is how an unrestricted key is written, not an error.
/// `None` means this kind has no permissions claim at all.
#[derive(Clone, Copy)]
enum EmbeddedPermissionEncoding {
    V1,
    V2,
}

fn permissions(
    encoding: Option<EmbeddedPermissionEncoding>,
    claim: Option<&str>,
) -> Result<Option<Permissions>, CliError> {
    let (Some(encoding), Some(claim)) = (encoding, claim) else {
        return Ok(None);
    };
    let permissions = match encoding {
        EmbeddedPermissionEncoding::V1 if claim.is_empty() => Permissions {
            super_user: Some(true),
            rules: None,
            conditions: None,
        },
        EmbeddedPermissionEncoding::V1 => {
            let bytes = BASE64.decode(claim).map_err(|error| {
                CliError::new("Could not base64-decode the permissions claim")
                    .with_details(format!("{error}"))
            })?;
            let permissions = PermissionsProtoV1::decode(bytes.as_slice()).map_err(|error| {
                CliError::new("Could not decode the v1 permissions protobuf")
                    .with_details(format!("{error}"))
            })?;
            Permissions::from_v1(permissions)?
        }
        EmbeddedPermissionEncoding::V2 => {
            let bytes = BASE64.decode(claim).map_err(|error| {
                CliError::new("Could not base64-decode the permissions claim")
                    .with_details(format!("{error}"))
            })?;
            let permissions = PermissionsProtoV2::decode(bytes.as_slice()).map_err(|error| {
                CliError::new("Could not decode the v2 permissions protobuf")
                    .with_details(format!("{error}"))
            })?;
            Permissions::from_v2(permissions)?
        }
    };
    Ok(Some(permissions))
}

#[cfg(test)]
mod tests {
    use super::*;
    use momento_protos::permission_rules::{permission_set::Kind as KindV2, ExplicitPermissions};

    /// A console key hides its JWT in an envelope; a bare token is itself.
    /// Getting this wrong means trying to decode the envelope as a JWT and
    /// reporting nonsense, so both shapes are pinned — along with the endpoint,
    /// which only the envelope carries.
    #[test]
    fn envelopes_are_unwrapped_and_bare_tokens_pass_through() {
        let jwt = "aaa.bbb.ccc";
        assert_eq!((None, jwt.to_owned()), unwrap_envelope(jwt));

        let envelope = STANDARD.encode(
            serde_json::json!({ "endpoint": "cell.momentohq.com", "api_key": jwt }).to_string(),
        );
        assert_eq!(
            (Some("cell.momentohq.com".to_owned()), jwt.to_owned()),
            unwrap_envelope(&envelope),
        );
    }

    /// An empty `p` is how an unrestricted key is written;
    /// a token kind without the claim has no permissions to show at all.
    /// The two must not collapse into each other.
    #[test]
    fn an_empty_permissions_claim_is_super_user_but_an_absent_one_is_nothing() {
        assert_eq!(
            None,
            permissions(Some(EmbeddedPermissionEncoding::V1), None).expect("no claim renders")
        );
        let super_user = permissions(Some(EmbeddedPermissionEncoding::V1), Some(""))
            .expect("empty claim renders");
        assert!(
            super_user.is_some_and(|value| value.super_user.expect("has super user field")),
            "an empty permissions claim should render as super-user",
        );
    }

    #[test]
    fn a_permissions_claim_is_ignored_when_the_token_format_does_not_carry_permissions() {
        assert_eq!(
            None,
            permissions(None, Some("not encoded permissions")).expect("claim is ignored")
        );
    }

    #[test]
    fn v2_permissions_are_decoded_with_the_v2_schema() {
        let permission_set = PermissionsProtoV2 {
            kind: Some(KindV2::Explicit(ExplicitPermissions { rules: vec![] })),
            conditions: vec![],
        };
        let claim = BASE64.encode(permission_set.encode_to_vec());
        let rendered = permissions(Some(EmbeddedPermissionEncoding::V2), Some(&claim))
            .expect("v2 claim should decode");

        assert_eq!(
            Some(Permissions {
                super_user: None,
                rules: Some(vec![]),
                conditions: Some(vec![])
            }),
            rendered
        );
    }
}
