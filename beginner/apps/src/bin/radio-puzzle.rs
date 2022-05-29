#![deny(unused_must_use)]
#![no_main]
#![no_std]

use core::str;

use cortex_m_rt::entry;
use dk::ieee802154::{Channel, Packet};

use core::time::Duration;
use heapless::LinearMap; // for storing character mappings
use heapless::Vec; // for storing byte arrays

// byte literal: b'c' vs. character literal 'c'

// this imports `beginner/apps/lib.rs` to retrieve our global logger + panicking-behavior
use apps as _;

const TEN_MS: u32 = 10_000;

#[entry]
fn main() -> ! {
    let board = dk::init().unwrap();
    let mut radio = board.radio;
    let mut timer = board.timer;

    // puzzle.hex uses channel 25 by default
    // NOTE if you ran `change-channel` then you may need to update the channel here
    radio.set_channel(Channel::_25); // <- must match the Dongle's listening channel

    let mut packet = Packet::new();

    let mut map: LinearMap<u8, u8, { 128 }> = LinearMap::new();
    let mut x: Vec<u8, 100> = Vec::new(); // can hold up to 8 elements

    let mut msg = b"a";

    for i in 0..=127 {
        let msg = &[i];
        packet.copy_from_slice(msg);
        defmt::println!(
            "sending: {}",
            str::from_utf8(msg).expect("msg was not valid UTF-8 data")
        );

        timer.wait(Duration::from_millis(20));
        radio.send(&mut packet);
        if radio.recv_timeout(&mut packet, &mut timer, TEN_MS).is_ok() {
            defmt::println!(
                "received: {}",
                str::from_utf8(&packet).expect("response was not valid UTF-8 data")
            );

            if packet.len() != 1 {
                panic!("packet len not 1");
            }
            map.insert(i, packet[0]).expect("dictionary full");
        } else {
            defmt::error!("no response or response packet was corrupted");
        }
    }

    // fetch the encrypoted string
    timer.wait(Duration::from_millis(20));
    let msg = b"";
    packet.copy_from_slice(msg);
    radio.send(&mut packet);
    let st: &str = if radio.recv_timeout(&mut packet, &mut timer, TEN_MS).is_ok() {
        let string1 = str::from_utf8(&packet).expect("response was not valid UTF-8 data");
        defmt::println!("received encrypted text: {}", string1.clone());
        string1
    } else {
        ""
    };

    for el in st.bytes() {
        for (key, val) in map.iter() {
            if val == &el {
                x.push(*key).unwrap();
            }
        }
    }

    let string2 = str::from_utf8(&x).expect("response was not valid UTF-8 data");
    defmt::println!("Decrypted text: {}", string2);

    dk::exit()
}
