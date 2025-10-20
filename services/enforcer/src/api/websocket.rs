use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{info, warn};
use uuid::Uuid;

use super::types::{DecisionEvent, StreamFilter};

#[derive(Deserialize)]
struct FilterMessage {
    r#type: String,
    #[serde(flatten)]
    filter: StreamFilter,
}

#[derive(Debug, Deserialize)]
pub struct StreamQuery {
    #[serde(default)]
    pub tenant_id: Option<String>,
    #[serde(default)]
    pub decision: Option<String>,
}

pub async fn ws_decision_stream(
    ws: WebSocketUpgrade,
    Query(query): Query<StreamQuery>,
    State((_policy_manager, event_tx)): State<(
        Arc<crate::policy::PolicyManager>,
        Arc<broadcast::Sender<DecisionEvent>>,
    )>,
) -> impl IntoResponse {
    let initial_filter = StreamFilter {
        tenant_id: query.tenant_id,
        decision: query.decision,
    };
    let event_tx = Arc::clone(&event_tx);

    ws.on_upgrade(move |socket| handle_decision_stream(socket, event_tx, initial_filter))
}

async fn handle_decision_stream(
    socket: WebSocket,
    event_tx: Arc<broadcast::Sender<DecisionEvent>>,
    initial_filter: StreamFilter,
) {
    let connection_id = Uuid::new_v4();
    info!(
        %connection_id,
        tenant_filter = initial_filter.tenant_id.as_deref().unwrap_or("*"),
        decision_filter = initial_filter.decision.as_deref().unwrap_or("*"),
        "decision stream connection established"
    );

    let (sink, mut stream) = socket.split();
    let (out_tx, mut out_rx) = mpsc::channel::<Message>(128);
    let filter_state = Arc::new(RwLock::new(initial_filter));

    let mut sink_task = tokio::spawn({
        let mut sink = sink;
        async move {
            while let Some(message) = out_rx.recv().await {
                if sink.send(message).await.is_err() {
                    break;
                }
            }
        }
    });

    let mut broadcast_task = tokio::spawn({
        let mut subscriber = event_tx.subscribe();
        let out_tx = out_tx.clone();
        let filter_state = Arc::clone(&filter_state);

        async move {
            if out_tx
                .send(Message::Text(
                    serde_json::json!({
                        "type": "connected",
                        "message": "Decision stream ready"
                    })
                    .to_string(),
                ))
                .await
                .is_err()
            {
                return;
            }

            loop {
                match subscriber.recv().await {
                    Ok(event) => {
                        let current_filter = { filter_state.read().await.clone() };
                        if should_send_event(&event, &current_filter) {
                            match serde_json::to_string(&serde_json::json!({
                                "type": "decision",
                                "data": event,
                            })) {
                                Ok(serialized) => {
                                    if out_tx.send(Message::Text(serialized)).await.is_err() {
                                        break;
                                    }
                                }
                                Err(err) => {
                                    warn!(error = ?err, "failed to serialize decision event")
                                }
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!(%connection_id, %skipped, "decision stream lagged; dropping events");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!(%connection_id, "decision stream broadcast channel closed");
                        break;
                    }
                }
            }
        }
    });

    let mut receive_task = tokio::spawn({
        let out_tx = out_tx.clone();
        let filter_state = Arc::clone(&filter_state);

        async move {
            while let Some(Ok(message)) = stream.next().await {
                match message {
                    Message::Ping(payload) => {
                        let _ = out_tx.send(Message::Pong(payload)).await;
                    }
                    Message::Text(text) => {
                        if let Ok(msg) = serde_json::from_str::<FilterMessage>(&text) {
                            if msg.r#type == "filter" {
                                *filter_state.write().await = msg.filter;
                            }
                        }
                    }
                    Message::Close(frame) => {
                        info!(%connection_id, ?frame, "decision stream client closed connection");
                        break;
                    }
                    Message::Pong(_) | Message::Binary(_) => {
                        // ignore other message types
                    }
                }
            }
        }
    });

    tokio::select! {
        _ = &mut sink_task => {},
        _ = &mut broadcast_task => {},
        _ = &mut receive_task => {},
    }

    sink_task.abort();
    broadcast_task.abort();
    receive_task.abort();

    info!(%connection_id, "decision stream connection closed");
}

fn should_send_event(event: &DecisionEvent, filter: &StreamFilter) -> bool {
    if let Some(ref tenant) = filter.tenant_id {
        if event.tenant_id != *tenant {
            return false;
        }
    }
    if let Some(ref decision) = filter.decision {
        let expected_allow = decision == "allow";
        if event.decision.allow != expected_allow {
            return false;
        }
    }
    true
}
