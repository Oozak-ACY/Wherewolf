//! Shared by the engine's test files.

use wherewolf_engine::{Command, Engine, Outputs};

/// The Table device says every line of the Moment being announced, if one
/// is (and the Game is not Paused), so the Moment opens as it would at the
/// table. For tests about what happens once a Moment is open.
pub fn announce(engine: &mut Engine, outputs: &mut Outputs) {
    let host = outputs
        .views()
        .find(|(_, v)| v.you == v.host)
        .filter(|(_, v)| !v.paused)
        .and_then(|(token, v)| Some((token.clone(), v.announcing?)));
    if let Some((host, moment)) = host {
        *outputs = engine
            .handle(&host, Command::Announced { moment })
            .expect("the Table device announces the current Moment");
    }
}
