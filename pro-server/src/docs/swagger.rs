use axum::{routing::get, Router, Json};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::models::*;
use crate::AppState;

#[derive(OpenApi)]
#[openapi(
    paths(
        // Auth
        auth_register,
        auth_login,
        auth_refresh,
        auth_logout,
        auth_me,
        auth_api_keys,
        auth_revoke_api_key,
        // Teams
        teams_create,
        teams_list,
        teams_get,
        teams_update,
        teams_delete,
        teams_members_list,
        teams_members_invite,
        teams_members_remove,
        teams_members_update_role,
        teams_invitations_accept,
        teams_invitations_decline,
        // Audit
        audit_query,
        audit_export,
        audit_stats,
        // RBAC
        rbac_roles_list,
        rbac_roles_create,
        rbac_roles_get,
        rbac_roles_update,
        rbac_roles_delete,
        rbac_permissions_list,
        rbac_check_permission,
        rbac_user_permissions,
        // Resources
        resources_servers_share,
        resources_servers_list,
        resources_servers_unshare,
        resources_snippets_create,
        resources_snippets_list,
        resources_snippets_get,
        resources_snippets_update,
        resources_snippets_delete,
        // SSO
        sso_config_create,
        sso_config_list,
        sso_config_get,
        sso_config_update,
        sso_config_delete,
        sso_saml_login,
        sso_saml_acs,
        sso_saml_metadata,
        sso_oidc_login,
        sso_oidc_callback,
    ),
    components(
        schemas(
            User,
            UserProfile,
            Team,
            TeamMember,
            TeamRole,
            Invitation,
            InvitationStatus,
            CreateTeamRequest,
            UpdateTeamRequest,
            InviteMemberRequest,
            Role,
            Permission,
            CheckPermissionRequest,
            CheckPermissionResponse,
            AuditLog,
            QueryAuditLogsRequest,
            AuditLogListResponse,
            SharedServer,
            ServerPermissions,
            Snippet,
            CreateSnippetRequest,
            UpdateSnippetRequest,
            ApiKey,
            CreateApiKeyRequest,
            CreateApiKeyResponse,
            SsoConfig,
            SsoProviderType,
            WebSocketMessage,
            PaginationParams,
            PaginationInfo,
            PaginatedResponse<serde_json::Value>,
            ErrorResponse,
            SuccessResponse<serde_json::Value>,
        )
    ),
    tags(
        (name = "Auth", description = "Authentication and API Key Management"),
        (name = "Teams", description = "Team Management"),
        (name = "Audit", description = "Audit Logs"),
        (name = "RBAC", description = "Role-Based Access Control"),
        (name = "Resources", description = "Shared Resources (Servers, Snippets)"),
        (name = "SSO", description = "SSO Integration (SAML, OIDC)"),
        (name = "WebSocket", description = "Real-time Notifications"),
    ),
    info(
        title = "EasySSH Pro API",
        version = "0.3.0",
        description = "RESTful API for EasySSH Pro - Team collaboration, audit logs, SSO",
        contact(name = "EasySSH Team", url = "https://easyssh.io"),
        license(name = "MIT", url = "https://opensource.org/licenses/MIT"),
    )
)]
pub struct ApiDoc;

// Placeholder handlers for OpenAPI documentation
#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    request_body = CreateUserRequest,
    responses(
        (status = 201, description = "User registered successfully", body = SuccessResponse<UserProfile>),
        (status = 400, description = "Invalid input", body = ErrorResponse),
    ),
    tag = "Auth"
)]
pub async fn auth_register() {}

#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid credentials", body = ErrorResponse),
    ),
    tag = "Auth"
)]
pub async fn auth_login() {}

#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Token refreshed", body = LoginResponse),
        (status = 401, description = "Invalid refresh token", body = ErrorResponse),
    ),
    tag = "Auth"
)]
pub async fn auth_refresh() {}

#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    responses(
        (status = 200, description = "Logout successful", body = SuccessResponse<()>),
    ),
    tag = "Auth",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn auth_logout() {}

#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    responses(
        (status = 200, description = "Current user info", body = SuccessResponse<UserProfile>),
    ),
    tag = "Auth",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn auth_me() {}

#[utoipa::path(
    post,
    path = "/api/v1/auth/api-keys",
    request_body = CreateApiKeyRequest,
    responses(
        (status = 201, description = "API key created", body = SuccessResponse<CreateApiKeyResponse>),
    ),
    tag = "Auth",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn auth_api_keys() {}

#[utoipa::path(
    delete,
    path = "/api/v1/auth/api-keys/{id}",
    params(
        ("id" = String, Path, description = "API key ID")
    ),
    responses(
        (status = 200, description = "API key revoked", body = SuccessResponse<()>),
        (status = 404, description = "API key not found", body = ErrorResponse),
    ),
    tag = "Auth",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn auth_revoke_api_key() {}

// Team endpoints
#[utoipa::path(
    post,
    path = "/api/v1/teams",
    request_body = CreateTeamRequest,
    responses(
        (status = 201, description = "Team created", body = SuccessResponse<Team>),
    ),
    tag = "Teams",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn teams_create() {}

#[utoipa::path(
    get,
    path = "/api/v1/teams",
    params(
        PaginationParams
    ),
    responses(
        (status = 200, description = "List of teams", body = SuccessResponse<PaginatedResponse<Team>>),
    ),
    tag = "Teams",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn teams_list() {}

#[utoipa::path(
    get,
    path = "/api/v1/teams/{id}",
    params(
        ("id" = String, Path, description = "Team ID")
    ),
    responses(
        (status = 200, description = "Team details", body = SuccessResponse<Team>),
        (status = 404, description = "Team not found", body = ErrorResponse),
    ),
    tag = "Teams",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn teams_get() {}

#[utoipa::path(
    put,
    path = "/api/v1/teams/{id}",
    params(
        ("id" = String, Path, description = "Team ID")
    ),
    request_body = UpdateTeamRequest,
    responses(
        (status = 200, description = "Team updated", body = SuccessResponse<Team>),
    ),
    tag = "Teams",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn teams_update() {}

#[utoipa::path(
    delete,
    path = "/api/v1/teams/{id}",
    params(
        ("id" = String, Path, description = "Team ID")
    ),
    responses(
        (status = 200, description = "Team deleted", body = SuccessResponse<()>),
    ),
    tag = "Teams",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn teams_delete() {}

#[utoipa::path(
    get,
    path = "/api/v1/teams/{id}/members",
    params(
        ("id" = String, Path, description = "Team ID")
    ),
    responses(
        (status = 200, description = "List of team members", body = SuccessResponse<Vec<TeamMember>>),
    ),
    tag = "Teams",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn teams_members_list() {}

#[utoipa::path(
    post,
    path = "/api/v1/teams/{id}/members",
    params(
        ("id" = String, Path, description = "Team ID")
    ),
    request_body = InviteMemberRequest,
    responses(
        (status = 201, description = "Invitation sent", body = SuccessResponse<Invitation>),
    ),
    tag = "Teams",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn teams_members_invite() {}

#[utoipa::path(
    delete,
    path = "/api/v1/teams/{id}/members/{member_id}",
    params(
        ("id" = String, Path, description = "Team ID"),
        ("member_id" = String, Path, description = "Member ID")
    ),
    responses(
        (status = 200, description = "Member removed", body = SuccessResponse<()>),
    ),
    tag = "Teams",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn teams_members_remove() {}

#[utoipa::path(
    put,
    path = "/api/v1/teams/{id}/members/{member_id}/role",
    params(
        ("id" = String, Path, description = "Team ID"),
        ("member_id" = String, Path, description = "Member ID")
    ),
    responses(
        (status = 200, description = "Member role updated", body = SuccessResponse<TeamMember>),
    ),
    tag = "Teams",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn teams_members_update_role() {}

#[utoipa::path(
    post,
    path = "/api/v1/teams/invitations/{token}/accept",
    params(
        ("token" = String, Path, description = "Invitation token")
    ),
    responses(
        (status = 200, description = "Invitation accepted", body = SuccessResponse<TeamMember>),
    ),
    tag = "Teams",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn teams_invitations_accept() {}

#[utoipa::path(
    post,
    path = "/api/v1/teams/invitations/{token}/decline",
    params(
        ("token" = String, Path, description = "Invitation token")
    ),
    responses(
        (status = 200, description = "Invitation declined", body = SuccessResponse<()>),
    ),
    tag = "Teams",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn teams_invitations_decline() {}

// Audit endpoints
#[utoipa::path(
    get,
    path = "/api/v1/audit",
    params(
        QueryAuditLogsRequest
    ),
    responses(
        (status = 200, description = "Audit logs", body = SuccessResponse<AuditLogListResponse>),
    ),
    tag = "Audit",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn audit_query() {}

#[utoipa::path(
    get,
    path = "/api/v1/audit/export",
    params(
        QueryAuditLogsRequest
    ),
    responses(
        (status = 200, description = "CSV export of audit logs", content_type = "text/csv"),
    ),
    tag = "Audit",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn audit_export() {}

#[utoipa::path(
    get,
    path = "/api/v1/audit/stats",
    params(
        QueryAuditLogsRequest
    ),
    responses(
        (status = 200, description = "Audit statistics", body = SuccessResponse<serde_json::Value>),
    ),
    tag = "Audit",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn audit_stats() {}

// RBAC endpoints
#[utoipa::path(
    get,
    path = "/api/v1/rbac/roles",
    params(
        PaginationParams
    ),
    responses(
        (status = 200, description = "List of roles", body = SuccessResponse<PaginatedResponse<Role>>),
    ),
    tag = "RBAC",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn rbac_roles_list() {}

#[utoipa::path(
    post,
    path = "/api/v1/rbac/roles",
    request_body = serde_json::Value,
    responses(
        (status = 201, description = "Role created", body = SuccessResponse<Role>),
    ),
    tag = "RBAC",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn rbac_roles_create() {}

#[utoipa::path(
    get,
    path = "/api/v1/rbac/roles/{id}",
    params(
        ("id" = String, Path, description = "Role ID")
    ),
    responses(
        (status = 200, description = "Role details", body = SuccessResponse<Role>),
    ),
    tag = "RBAC",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn rbac_roles_get() {}

#[utoipa::path(
    put,
    path = "/api/v1/rbac/roles/{id}",
    params(
        ("id" = String, Path, description = "Role ID")
    ),
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Role updated", body = SuccessResponse<Role>),
    ),
    tag = "RBAC",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn rbac_roles_update() {}

#[utoipa::path(
    delete,
    path = "/api/v1/rbac/roles/{id}",
    params(
        ("id" = String, Path, description = "Role ID")
    ),
    responses(
        (status = 200, description = "Role deleted", body = SuccessResponse<()>),
    ),
    tag = "RBAC",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn rbac_roles_delete() {}

#[utoipa::path(
    get,
    path = "/api/v1/rbac/permissions",
    responses(
        (status = 200, description = "List of permissions", body = SuccessResponse<Vec<Permission>>),
    ),
    tag = "RBAC",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn rbac_permissions_list() {}

#[utoipa::path(
    post,
    path = "/api/v1/rbac/check",
    request_body = CheckPermissionRequest,
    responses(
        (status = 200, description = "Permission check result", body = SuccessResponse<CheckPermissionResponse>),
    ),
    tag = "RBAC",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn rbac_check_permission() {}

#[utoipa::path(
    get,
    path = "/api/v1/rbac/user/permissions",
    responses(
        (status = 200, description = "User permissions", body = SuccessResponse<Vec<String>>),
    ),
    tag = "RBAC",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn rbac_user_permissions() {}

// Resource endpoints
#[utoipa::path(
    post,
    path = "/api/v1/resources/servers",
    request_body = serde_json::Value,
    responses(
        (status = 201, description = "Server shared", body = SuccessResponse<SharedServer>),
    ),
    tag = "Resources",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn resources_servers_share() {}

#[utoipa::path(
    get,
    path = "/api/v1/resources/servers",
    responses(
        (status = 200, description = "List of shared servers", body = SuccessResponse<Vec<SharedServer>>),
    ),
    tag = "Resources",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn resources_servers_list() {}

#[utoipa::path(
    delete,
    path = "/api/v1/resources/servers/{id}",
    params(
        ("id" = String, Path, description = "Server share ID")
    ),
    responses(
        (status = 200, description = "Server unshared", body = SuccessResponse<()>),
    ),
    tag = "Resources",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn resources_servers_unshare() {}

#[utoipa::path(
    post,
    path = "/api/v1/resources/snippets",
    request_body = CreateSnippetRequest,
    responses(
        (status = 201, description = "Snippet created", body = SuccessResponse<Snippet>),
    ),
    tag = "Resources",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn resources_snippets_create() {}

#[utoipa::path(
    get,
    path = "/api/v1/resources/snippets",
    responses(
        (status = 200, description = "List of snippets", body = SuccessResponse<Vec<Snippet>>),
    ),
    tag = "Resources",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn resources_snippets_list() {}

#[utoipa::path(
    get,
    path = "/api/v1/resources/snippets/{id}",
    params(
        ("id" = String, Path, description = "Snippet ID")
    ),
    responses(
        (status = 200, description = "Snippet details", body = SuccessResponse<Snippet>),
    ),
    tag = "Resources",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn resources_snippets_get() {}

#[utoipa::path(
    put,
    path = "/api/v1/resources/snippets/{id}",
    params(
        ("id" = String, Path, description = "Snippet ID")
    ),
    request_body = UpdateSnippetRequest,
    responses(
        (status = 200, description = "Snippet updated", body = SuccessResponse<Snippet>),
    ),
    tag = "Resources",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn resources_snippets_update() {}

#[utoipa::path(
    delete,
    path = "/api/v1/resources/snippets/{id}",
    params(
        ("id" = String, Path, description = "Snippet ID")
    ),
    responses(
        (status = 200, description = "Snippet deleted", body = SuccessResponse<()>),
    ),
    tag = "Resources",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn resources_snippets_delete() {}

// SSO endpoints
#[utoipa::path(
    post,
    path = "/api/v1/sso/config",
    request_body = serde_json::Value,
    responses(
        (status = 201, description = "SSO config created", body = SuccessResponse<SsoConfig>),
    ),
    tag = "SSO",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn sso_config_create() {}

#[utoipa::path(
    get,
    path = "/api/v1/sso/config",
    responses(
        (status = 200, description = "List of SSO configs", body = SuccessResponse<Vec<SsoConfig>>),
    ),
    tag = "SSO",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn sso_config_list() {}

#[utoipa::path(
    get,
    path = "/api/v1/sso/config/{id}",
    params(
        ("id" = String, Path, description = "SSO Config ID")
    ),
    responses(
        (status = 200, description = "SSO config details", body = SuccessResponse<SsoConfig>),
    ),
    tag = "SSO",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn sso_config_get() {}

#[utoipa::path(
    post,
    path = "/api/v1/sso/config/{id}",
    params(
        ("id" = String, Path, description = "SSO Config ID")
    ),
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "SSO config updated", body = SuccessResponse<SsoConfig>),
    ),
    tag = "SSO",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn sso_config_update() {}

#[utoipa::path(
    delete,
    path = "/api/v1/sso/config/{id}",
    params(
        ("id" = String, Path, description = "SSO Config ID")
    ),
    responses(
        (status = 200, description = "SSO config deleted", body = SuccessResponse<()>),
    ),
    tag = "SSO",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn sso_config_delete() {}

#[utoipa::path(
    get,
    path = "/sso/saml/{team_id}/login",
    params(
        ("team_id" = String, Path, description = "Team ID")
    ),
    responses(
        (status = 302, description = "Redirect to IdP"),
    ),
    tag = "SSO"
)]
pub async fn sso_saml_login() {}

#[utoipa::path(
    post,
    path = "/sso/saml/{team_id}/acs",
    params(
        ("team_id" = String, Path, description = "Team ID")
    ),
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "SAML authentication successful", body = SuccessResponse<LoginResponse>),
    ),
    tag = "SSO"
)]
pub async fn sso_saml_acs() {}

#[utoipa::path(
    get,
    path = "/sso/saml/{team_id}/metadata",
    params(
        ("team_id" = String, Path, description = "Team ID")
    ),
    responses(
        (status = 200, description = "SAML metadata XML", content_type = "application/xml"),
    ),
    tag = "SSO"
)]
pub async fn sso_saml_metadata() {}

#[utoipa::path(
    get,
    path = "/sso/oidc/{team_id}/login",
    params(
        ("team_id" = String, Path, description = "Team ID")
    ),
    responses(
        (status = 302, description = "Redirect to IdP"),
    ),
    tag = "SSO"
)]
pub async fn sso_oidc_login() {}

#[utoipa::path(
    get,
    path = "/sso/oidc/{team_id}/callback",
    params(
        ("team_id" = String, Path, description = "Team ID"),
        ("code" = String, Query, description = "Authorization code"),
        ("state" = Option<String>, Query, description = "State parameter")
    ),
    responses(
        (status = 200, description = "OIDC authentication successful", body = SuccessResponse<LoginResponse>),
    ),
    tag = "SSO"
)]
pub async fn sso_oidc_callback() {}

pub fn swagger_routes() -> Router<AppState> {
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/api-docs/openapi.json", get(openapi_json))
}

async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}