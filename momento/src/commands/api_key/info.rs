use super::permission_protos::{super_user, PermissionsProtoV1, PermissionsProtoV2};
use crate::commands::custom_role::utils::Permissions;
use crate::error::CliError;

use base64::{
    engine::{general_purpose::STANDARD, GeneralPurpose},
    Engine,
};
use prost::Message;
use serde::Deserialize;
use serde_json::Value;

const BASE64: GeneralPurpose = GeneralPurpose::new(
    &base64::alphabet::URL_SAFE,
    base64::engine::general_purpose::GeneralPurposeConfig::new()
        .with_decode_padding_mode(base64::engine::DecodePaddingMode::Indifferent),
);

fn get_claim_string(claims: &Value, name: &str) -> String {
    claims
        .get(name)
        .and_then(Value::as_str)
        .unwrap_or("(missing)")
        .to_string()
}

/// Decodes one dot-separated part of a JWT (0 = header, 1 = payload) to JSON.
fn decode_segment(jwt: &str, index: usize, name: &str) -> Result<Value, CliError> {
    let segment = jwt
        .split('.')
        .nth(index)
        .ok_or(CliError::new("Not a JWT; expected dot-separated parts"))?;
    let bytes = BASE64
        .decode(segment)
        .map_err(|error| CliError::new(format!("JWT {name} is not base64 ({error})")))?;
    serde_json::from_slice::<Value>(&bytes)
        .map_err(|error| CliError::new(format!("JWT {name} is not JSON ({error})")))
}

struct Classified {
    kind: &'static str,
    identity_label: &'static str,
    identity_value: String,
    permission_encoding: Option<EmbeddedPermissionEncoding>,
}

fn classify(claims: &Value, auth_token: &str) -> Result<Classified, CliError> {
    let token_type = claims.get("t").and_then(Value::as_str);
    let version = claims.get("ver").and_then(Value::as_i64);
    match token_type {
        Some("disposable") => match version {
            Some(1) => Ok(Classified {
                kind: "Disposable token (v1)",
                identity_label: "Customer",
                identity_value: get_claim_string(claims, "sub"), // subject
                permission_encoding: Some(EmbeddedPermissionEncoding::V1),
            }),
            Some(2) => Ok(Classified {
                kind: "Disposable token (v2)",
                identity_label: "Account",
                identity_value: get_claim_string(claims, "a"), // account ID
                permission_encoding: Some(EmbeddedPermissionEncoding::V2),
            }),
            Some(version) => Err(CliError::new(format!(
                "Unsupported disposable token version {version}"
            ))),
            None => Err(CliError::new("No version for disposable token")),
        },
        Some("g") => Ok(Classified {
            kind: "Global API key",
            identity_label: "Key",
            identity_value: get_claim_string(claims, "jti"), // JWT ID
            permission_encoding: None,
        }),
        Some("gr") => Ok(Classified {
            kind: "Global API key refresh token",
            identity_label: "Key",
            identity_value: get_claim_string(claims, "akid"), // API key ID (not refresh token ID)
            permission_encoding: None,
        }),
        Some(other) => Err(CliError::new(format!("Unknown token type {other}"))),
        None => match claims.get("iss") {
            Some(issuer) => Err(CliError::new(format!("Unsupported issuer: {issuer}"))),
            None => {
                if version == Some(1) {
                    Ok(Classified {
                        kind: "API token (v1)",
                        identity_label: "Customer",
                        identity_value: get_claim_string(claims, "sub"), // subject
                        permission_encoding: Some(EmbeddedPermissionEncoding::V1),
                    })
                } else if claims.get("cp").is_some() {
                    // cp (control_plane_proxy_endpoint) only appears on legacy tokens
                    Ok(Classified {
                        kind: "Legacy token",
                        identity_label: "Customer",
                        identity_value: get_claim_string(claims, "sub"), // subject
                        permission_encoding: None,
                    })
                } else {
                    // The only way to identify a customer signed token is by checking that
                    // a keyid exists in the header, and none of the other claim tags identifying
                    // other token types were found.
                    let header = decode_segment(auth_token, 0, "header")?;
                    let key_id = header
                        .get("kid")
                        .and_then(Value::as_str)
                        .ok_or(CliError::new("Missing key id"))?
                        .to_string();
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

/// A token's permissions claim.
pub enum EmbeddedPermissions {
    Absent,
    Decoded(Permissions),
    Undecodable(CliError),
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
    pub expires: Option<Value>,
    pub permissions: EmbeddedPermissions,
    pub claims: Value,
}

/// Offline and unverified: nothing is contacted and no signature is checked.
/// That is the point — a token that fails validation can still decode here,
/// which is what you need when one is behaving unexpectedly.
pub fn decode(key: &str) -> Result<DecodedApiKey, CliError> {
    let (endpoint, jwt) = unwrap_envelope(key);

    let claims = decode_segment(&jwt, 1, "payload")
        .map_err(|error| CliError::new(format!("Could not decode token: {error}")))?;

    let classified = classify(&claims, &jwt).unwrap_or_else(|error| Classified {
        kind: "(unknown)",
        identity_label: "Unrecognized token type",
        identity_value: format!("{error}"),
        permission_encoding: None,
    });

    let expires = claims.get("exp");
    let permissions = permissions(
        classified.permission_encoding,
        claims.get("p").and_then(Value::as_str),
    );

    Ok(DecodedApiKey {
        kind: classified.kind,
        identity_label: classified.identity_label,
        identity_value: classified.identity_value,
        endpoint,
        expires: expires.cloned(),
        permissions,
        claims,
    })
}

/// Console- and SDK-issued keys arrive as base64 JSON,
/// carrying the cell endpoint alongside the JWT.
fn unwrap_envelope(key: &str) -> (Option<String>, String) {
    #[derive(Deserialize)]
    struct Envelope {
        api_key: String,
        endpoint: Option<String>,
    }
    match STANDARD
        .decode(key)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Envelope>(&bytes).ok())
    {
        Some(envelope) => (envelope.endpoint, envelope.api_key),
        None => (None, key.to_owned()),
    }
}

#[derive(Clone, Copy)]
enum EmbeddedPermissionEncoding {
    V1,
    V2,
}

fn permissions(
    encoding: Option<EmbeddedPermissionEncoding>,
    claim: Option<&str>,
) -> EmbeddedPermissions {
    let (Some(encoding), Some(claim)) = (encoding, claim) else {
        return EmbeddedPermissions::Absent;
    };
    match decode_permissions(encoding, claim) {
        Ok(permissions) => EmbeddedPermissions::Decoded(permissions),
        Err(error) => EmbeddedPermissions::Undecodable(error),
    }
}

fn decode_permissions(
    encoding: EmbeddedPermissionEncoding,
    claim: &str,
) -> Result<Permissions, CliError> {
    if claim.is_empty() && matches!(encoding, EmbeddedPermissionEncoding::V1) {
        return Ok(super_user());
    }
    let bytes = BASE64.decode(claim).map_err(|error| {
        CliError::new("Could not base64-decode the permissions claim")
            .with_details(format!("{error}"))
    })?;
    let permissions = match encoding {
        EmbeddedPermissionEncoding::V1 => Permissions::from_v1(
            PermissionsProtoV1::decode(bytes.as_slice()).map_err(|error| {
                CliError::new("Could not decode the v1 permissions protobuf")
                    .with_details(format!("{error}"))
            })?,
        )?,
        EmbeddedPermissionEncoding::V2 => Permissions::from_v2(
            PermissionsProtoV2::decode(bytes.as_slice()).map_err(|error| {
                CliError::new("Could not decode the v2 permissions protobuf")
                    .with_details(format!("{error}"))
            })?,
        )?,
    };
    Ok(permissions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use momento_protos::permission_rules::{permission_set::Kind as KindV2, ExplicitPermissions};

    #[test]
    fn test_unwrap_envelope_and_preserve_bare_token() {
        let jwt = "aaa.bbb.ccc";
        assert_eq!((None, jwt.to_owned()), unwrap_envelope(jwt));

        let envelope = STANDARD.encode(
            serde_json::json!({ "endpoint": "cell.momentohq.com", "api_key": jwt }).to_string(),
        );
        assert_eq!(
            (Some("cell.momentohq.com".to_owned()), jwt.to_owned()),
            unwrap_envelope(&envelope),
        );

        let without_endpoint = STANDARD.encode(serde_json::json!({ "api_key": jwt }).to_string());
        assert_eq!((None, jwt.to_owned()), unwrap_envelope(&without_endpoint));
    }

    /// A token without the `p` claim is fully restricted,
    /// while a v1 with an empty `p` is fully unrestricted.
    #[test]
    fn test_permissions_with_no_claim_vs_empty_claim() {
        assert!(matches!(
            permissions(Some(EmbeddedPermissionEncoding::V1), None),
            EmbeddedPermissions::Absent
        ));

        let EmbeddedPermissions::Decoded(super_user) =
            permissions(Some(EmbeddedPermissionEncoding::V1), Some(""))
        else {
            panic!("empty claim should decode");
        };
        assert_eq!(
            Permissions {
                super_user: Some(true),
                rules: None,
                conditions: None,
            },
            super_user
        );
    }

    #[test]
    fn test_permissions_for_token_kind_without_permissions() {
        assert!(matches!(
            permissions(None, Some("not encoded permissions")),
            EmbeddedPermissions::Absent
        ));
    }

    #[test]
    fn test_permissions_with_unrecognized_claim_is_reported_not_fatal() {
        let permission_set = PermissionsProtoV2 {
            kind: None,
            conditions: vec![],
        };
        let claim = BASE64.encode(permission_set.encode_to_vec());
        assert!(matches!(
            permissions(Some(EmbeddedPermissionEncoding::V2), Some(&claim)),
            EmbeddedPermissions::Undecodable(_)
        ));
    }

    #[test]
    fn test_permissions_with_v2_schema() {
        let permission_set = PermissionsProtoV2 {
            kind: Some(KindV2::Explicit(ExplicitPermissions { rules: vec![] })),
            conditions: vec![],
        };
        let claim = BASE64.encode(permission_set.encode_to_vec());
        let EmbeddedPermissions::Decoded(rendered) =
            permissions(Some(EmbeddedPermissionEncoding::V2), Some(&claim))
        else {
            panic!("v2 claim should decode");
        };

        assert_eq!(
            Permissions {
                super_user: None,
                rules: Some(vec![]),
                conditions: Some(vec![])
            },
            rendered
        );
    }
}
