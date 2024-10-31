use mpris::{Player, PlayerFinder};

fn get_active_player() -> Option<Player> {
  match PlayerFinder::new() {
    Ok(finder) => match finder.find_active() {
      Ok(player) => {
        return Some(player);
      }
      Err(mpris::FindingError::DBusError(e)) => {
        log::error!("[PCTL] Error finding active player (DBus Error): {:#?}", e);
      },
      Err(mpris::FindingError::NoPlayerFound) => {},
    },
    Err(err) => {
      log::error!("[PCTL] Failed to create PlayerFinder (DBus Error): {:#?}", err);
    }
  };
  None
}

pub fn pause_active() {
  match get_active_player() {
    Some(player) => {
      match player.get_playback_status() {
        Ok(playback) => {
          if playback == mpris::PlaybackStatus::Playing {
            if let Err(e) = player.pause() {
              log::error!("[PCTL] Error pausing player: {:#?}", e);
            }
          }
        },
        Err(e) => {
          log::error!("[PCTL] Error getting playback status: {:#?}", e);
        }
      }
    },
    None => {},
  }
}

pub fn resume_active() {
  match get_active_player() {
    Some(player) => {
      match player.get_playback_status() {
        Ok(playback) => {
          if playback == mpris::PlaybackStatus::Paused {
            if let Err(e) = player.play() {
              log::error!("[PCTL] Error resuming player: {:#?}", e);
            }
          }
        },
        Err(e) => {
          log::error!("[PCTL] Error getting playback status: {:#?}", e);
        }
      }
    },
    None => {},
  }
}
