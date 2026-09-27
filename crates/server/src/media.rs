//! The media adapter (ADR 0001, server-side lever): keeps each Player's
//! `canSubscribe` in the Lobby's LiveKit room in line with the engine's
//! visibility plan. The client applies the other lever itself, from the
//! `audience` in its view.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures_util::future::join_all;
use tokio::sync::mpsc;
use wherewolf_engine::Outputs;

use crate::call::CallConfig;

/// While someone must receive nobody, the room is checked this often, so a
/// Player who rejoins the call with an older ticket is shut out again quickly.
const RECONCILE_EVERY: Duration = Duration::from_secs(2);

/// For each Player (by call identity), whether they may receive anyone at all,
/// which is their `canSubscribe` in the call.
type Plan = HashMap<String, bool>;

/// One Lobby's link to its room. Dropping it stops the background worker.
pub struct Media {
    plans: mpsc::UnboundedSender<Plan>,
}

impl Media {
    pub fn new(call: Arc<CallConfig>, room: String) -> Self {
        let (plans, inbox) = mpsc::unbounded_channel();
        tokio::spawn(apply_plans(call, room, inbox));
        Self { plans }
    }

    /// Hands over the plan from the engine's latest views.
    pub fn follow(&self, outputs: &Outputs) {
        let plan = outputs
            .views()
            .map(|(_, view)| (view.you.to_string(), view.receives_anyone()))
            .collect();
        let _ = self.plans.send(plan);
    }
}

/// Applies each new plan as soon as it arrives (only the Players whose need
/// changed), and reconciles the room while anyone must receive nobody, or
/// until a switch that failed has been set right.
async fn apply_plans(
    call: Arc<CallConfig>,
    room: String,
    mut plans: mpsc::UnboundedReceiver<Plan>,
) {
    let mut current = Plan::new();
    // A switch failed: the room may not follow the plan yet.
    let mut unsure = false;
    let mut tick = tokio::time::interval(RECONCILE_EVERY);
    loop {
        let someone_shut_out = current.values().any(|&can_subscribe| !can_subscribe);
        tokio::select! {
            next = plans.recv() => {
                let Some(next) = next else { return };
                let changed: Vec<(String, bool)> = next
                    .iter()
                    .filter(|(identity, can_subscribe)| current.get(*identity) != Some(can_subscribe))
                    .map(|(identity, &can_subscribe)| (identity.clone(), can_subscribe))
                    .collect();
                current = next;
                unsure |= !switch(&call, &room, changed).await;
            }
            _ = tick.tick(), if someone_shut_out || unsure => {
                match call.subscribers(&room).await {
                    Ok(in_room) => {
                        let wrong = in_room
                            .into_iter()
                            .filter_map(|(identity, can_subscribe)| {
                                let wanted = *current.get(&identity)?;
                                (can_subscribe != wanted).then_some((identity, wanted))
                            })
                            .collect();
                        unsure = !switch(&call, &room, wrong).await;
                    }
                    Err(error) => tracing::warn!(room, error, "could not list the call"),
                }
            }
        }
    }
}

/// Switches these Players' `canSubscribe` all at once, and tells whether every
/// switch went through. A Player not in the call yet fails harmlessly: their
/// ticket already carries the plan, and the next reconciliation skips them.
async fn switch(call: &CallConfig, room: &str, changes: Vec<(String, bool)>) -> bool {
    let updates = changes.iter().map(|(identity, can_subscribe)| async move {
        match call.set_can_subscribe(room, identity, *can_subscribe).await {
            Ok(()) => {
                tracing::debug!(room, identity, can_subscribe, "switched");
                true
            }
            Err(error) => {
                tracing::debug!(room, identity, error, "could not switch canSubscribe");
                false
            }
        }
    });
    join_all(updates).await.into_iter().all(|ok| ok)
}
