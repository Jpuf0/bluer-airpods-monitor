use std::{cmp::min, io::ErrorKind, sync::Arc, time::Duration};

use bluer::{l2cap::{SeqPacket, Socket, SocketAddr}, Adapter, Address, Result};
use tokio::{sync::{broadcast, Mutex}, time::sleep};

pub struct AAPSocketInner {
  current_anc: ANC,
  ears_in: (bool, bool),
  event_tx: broadcast::Sender<AAPEvent>,
  batteries: BatteryState,
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChargingState {
  #[default]
  Unknown,
  Charging,
  NotCharging,
  Disconnected,
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BatteryState {
  pub single: Option<(u8, ChargingState)>,
  pub left: Option<(u8, ChargingState)>,
  pub right: Option<(u8, ChargingState)>,
  pub case: Option<(u8, ChargingState)>,
}

#[derive(Clone)]
pub struct AAPSocket(Arc<Mutex<AAPSocketInner>>, Arc<SeqPacket>);

impl AAPSocket {
    pub fn socket(&self) -> &Arc<Mutex<AAPSocketInner>> {
        &self.0
    }

    pub fn stream(&self) -> &Arc<SeqPacket> {
        &self.1
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ANC {
  Off,
  NoiseCanceling,
  Transparency,
  Adaptive,
}

impl ANC {
  fn to_u8(&self) -> u8 {
    match self {
      ANC::Off => 0x01,
      ANC::NoiseCanceling => 0x02,
      ANC::Transparency => 0x03,
      ANC::Adaptive => 0x04,
    }
  }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AAPEvent {
  ANCChanged(ANC),
  EarsChanged((bool, bool)),
  BatteriesChanged(BatteryState),
  Disconnected,
}

impl AAPSocket {
  pub async fn init(adapter: Adapter, address: Address) -> Result<AAPSocket> {
    let socket = Socket::new_seq_packet().unwrap();
    socket.bind(SocketAddr::new(adapter.address().await.unwrap(), bluer::AddressType::BrEdr, 0)).unwrap();

    let sa = SocketAddr::new(address, bluer::AddressType::BrEdr, 0x1001);

    let stream = socket.connect(sa).await.expect("Failed to connect");

    let mtu = stream.as_ref().recv_mtu().unwrap();

    let (event_tx, _) = broadcast::channel::<AAPEvent>(16);

    let s = Self(
      Arc::new(Mutex::new(AAPSocketInner {
        current_anc: ANC::Off,
        ears_in: (false, false),
        event_tx: event_tx.clone(),
        batteries: Default::default(),
      })),
      Arc::new(stream)
    );


    let s2: AAPSocket = s.clone();

    sleep(Duration::from_secs(1)).await;
    s.send(&vec![0x00, 0x00, 0x04, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]).await?; // init command

    tokio::task::spawn(async move {
      let s = s2;

      loop {
        let mut buf = vec![ 0u8; mtu.into() ];

        match s.stream().recv(&mut buf).await {
          Ok(bytes) => {
            let buf = &buf[0..bytes];

            if buf.len() >= 5 {
              if buf[4] == 0x09 { // Settings
                if buf[6] == 0x0D { // ANC
                  let anc = match buf[7] {
                    1 => ANC::Off,
                    2 => ANC::NoiseCanceling,
                    3 => ANC::Transparency,
                    4 => ANC::Adaptive,
                    _ => {
                      log::warn!("Unknown ANC value: {:?}", buf[7]);
                      ANC::Off
                    }
                  };

                  {
                    let mut s = s.socket().lock().await;
                    if !(anc == ANC::Off && (s.current_anc == ANC::Off || s.current_anc == ANC::Adaptive)) {
                      s.current_anc = anc;
                    }
                  };

                  s.socket().lock().await.current_anc = anc;
                  let _ = event_tx.send(AAPEvent::ANCChanged(anc));
                } else {
                  log::warn!("Unknown Settings: {:?}", buf[6]);
                }
              } else if buf[4] == 0x04 { // Battery
                let mut state = BatteryState::default();
                let count = buf[6];

                for i in 0..count {
                  let start_byte = 7 + i as usize * 5;

                  let charge = min(100, buf[start_byte + 2]);
                  let charging = match buf[start_byte + 3] {
                    0 => ChargingState::Unknown,
                    1 => ChargingState::Charging,
                    2 => ChargingState::NotCharging,
                    3 => ChargingState::Disconnected,
                    _ => {
                      log::warn!("Unknown charging state: {:?}", buf[start_byte + 3]);
                      ChargingState::Unknown
                    }
                  };

                  let data = Some((charge, charging));

                  match buf[start_byte] {
                    0x01 => state.single = data,
                    0x02 => state.right = data,
                    0x03 => state.left = data,
                    0x04 => state.case = data,
                    _ => {
                      log::warn!("Unknown battery: {:?}", buf[start_byte]);
                    }
                  }
                }

                s.socket().lock().await.batteries = state;
                let _ = event_tx.send(AAPEvent::BatteriesChanged(state));
              } else if buf[4] == 0x06 { // Ears
                let new = (buf[7] == 0, buf[6] == 0);
                s.socket().lock().await.ears_in = new;
                let _ = event_tx.send(AAPEvent::EarsChanged(new));
              } else {
                if buf.len() >= 30 {
                  log::warn!("misc len >= 5 packet received: command {} len {}", buf[4], buf.len());
                } else {
                  log::warn!("misc len >= 5 packet received: command {} {:X?}", buf[4], buf);
                }
              }
            } else {
              log::warn!("Too short packet received: {:X?}", buf);
            }
          },
          Err(err) => {
            match err.kind() {
              ErrorKind::ConnectionReset => {
                let _ = event_tx.send(AAPEvent::Disconnected);
              },
              _ => {
                log::error!("Something went wrong: {:#?}", err);
              }
            }
            break;
          }
        }
      }
    });

    sleep(Duration::from_secs(1)).await;
    s.enable_notifications().await?;

    Ok(s)
  }

  async fn send(&self, data: &[u8]) -> Result<()> {
    self.stream().send(data).await?;
    Ok(())
  }

  pub async fn enable_notifications(&self) -> Result<()> {
    self.send(&vec![0x04, 0x00, 0x04, 0x00, 0x0f, 0x00, 0xff, 0xff, 0xff, 0xff]).await?;
    Ok(())
  }

  pub async fn set_anc(&self, anc: ANC) -> Result<()> {
    self.send(&vec![0x04, 0x00, 0x04, 0x00, 0x09, 0x00, 0x0D, anc.to_u8(), 0x00, 0x00, 0x00]).await?;
    self.socket().lock().await.current_anc = anc;
    Ok(())
  }

  pub async fn get_anc(&self) -> ANC {
    self.socket().lock().await.current_anc
  }

  pub async fn get_batteries(&self) -> BatteryState {
    self.socket().lock().await.batteries
  }

  pub async fn subscribe(&self) -> broadcast::Receiver<AAPEvent> {
    self.socket().lock().await.event_tx.subscribe()
  }
}
