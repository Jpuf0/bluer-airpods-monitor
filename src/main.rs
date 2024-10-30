use bluer::{
    l2cap::{SocketAddr, Stream, PSM_LE_DYN_START},
    Address, AddressType,
};
use rand::prelude::*;
use std::{env, process::exit};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> bluer::Result<()> {
    env_logger::init();
    let session = bluer::Session:new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;

    let args: Vec<_> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: bluer-l2cap-server <psm>");
        exit(1);
    }

    let target_addr: Address = args[1].parse().expect("Invalid address");
    let target_sa = SocketAddr::new(target_addr, AddressType::LePublic, PSM);
    
    println!("Connecting to {:?}", &target_sa);
    let mut stream = Stream::connect(target_sa).await.expect("Failed to connect");
    println!("Local address: {:?}", stream.as_ref().local_addr()?);
    println!("Remote address: {:?}", stream.peer_addr()?);
    println!("Send MTU: {:?}", stream.as_ref().send_mtu());
    println!("Recv MTU: {}", stream.as_ref().recv_mtu()?);
    println!("Security: {:?}", stream.as_ref().security()?);
    println!("Flow control: {:?}", stream.as_ref().flow_control());
}