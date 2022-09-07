#![no_main]
#![no_std]

// this imports `beginner/apps/lib.rs` to retrieve our global logger + panicking-behavior
use firmware as _;
//use heapless::consts::*;
//use heapless::Vec;

#[rtic::app(device = dk, peripherals = false)]
mod app {

    use core::num::NonZeroU8;

    use dk::{
        peripheral::USBD,
        usbd::{self, Ep0In, Event},
    };
    use usb2::State;
    // HEADS UP to use *your* USB packet parser uncomment line 12 and remove line 13
    // use usb::{Request, Descriptor};
    use usb2::{GetDescriptor as Descriptor, StandardRequest as Request};

    #[local]
    struct MyLocalResources {
        usbd: USBD,
        ep0in: Ep0In,
        state: State,
    }

    #[shared]
    struct MySharedResources {}

    #[init]
    fn init(_cx: init::Context) -> (MySharedResources, MyLocalResources, init::Monotonics) {
        let board = dk::init().unwrap();

        usbd::init(board.power, &board.usbd);

        (
            MySharedResources {},
            MyLocalResources {
                usbd: board.usbd,
                ep0in: board.ep0in,
                state: State::Default,
            },
            init::Monotonics(),
        )
    }

    #[task(binds = USBD, local = [usbd, ep0in, state])]
    fn main(cx: main::Context) {
        let usbd = cx.local.usbd;
        let ep0in = cx.local.ep0in;
        let state = cx.local.state;

        while let Some(event) = usbd::next_event(usbd) {
            on_event(usbd, ep0in, state, event)
        }
    }

    fn on_event(usbd: &USBD, ep0in: &mut Ep0In, state: &mut State, event: Event) {
        defmt::println!("USB: {} @ {}", event, dk::uptime());

        match event {
            // TODO change `state` as specified in chapter 9.1 USB Device States, of the USB specification
            Event::UsbReset => {
                defmt::println!("USB reset condition detected");
                //todo!();
                *state = State::Default;
            }

            Event::UsbEp0DataDone => {
                defmt::println!("EP0IN: transfer complete");
                ep0in.end(usbd);
            }

            Event::UsbEp0Setup => {
                if ep0setup(usbd, ep0in, state).is_err() {
                    // unsupported or invalid request:
                    // TODO: add code to stall the endpoint
                    defmt::warn!("EP0IN: unexpected request; stalling the endpoint");
                }
            }
        }
    }

    fn ep0setup(usbd: &USBD, ep0in: &mut Ep0In, state: &mut State) -> Result<(), ()> {
        let bmrequesttype = usbd.bmrequesttype.read().bits() as u8;
        let brequest = usbd.brequest.read().brequest().bits();
        let wlength = usbd::wlength(usbd);
        let windex = usbd::windex(usbd);
        let wvalue = usbd::wvalue(usbd);

        defmt::println!(
            "bmrequesttype: {}, brequest: {}, wlength: {}, windex: {}, wvalue: {}",
            bmrequesttype,
            brequest,
            wlength,
            windex,
            wvalue
        );

        let request = Request::parse(bmrequesttype, brequest, wvalue, windex, wlength)
            .expect("Error parsing request");
        defmt::println!("EP0: {}", defmt::Debug2Format(&request));
        //                        ^^^^^^^^^^^^^^^^^^^ this adapter is currently needed to log
        //                                            `StandardRequest` with `defmt`

        match request {
            Request::GetDescriptor { descriptor, length } => match descriptor {
                Descriptor::Device => {
                    let desc = usb2::device::Descriptor {
                        bDeviceClass: 0,
                        bDeviceProtocol: 0,
                        bDeviceSubClass: 0,
                        bMaxPacketSize0: usb2::device::bMaxPacketSize0::B64,
                        bNumConfigurations: core::num::NonZeroU8::new(1).unwrap(),
                        bcdDevice: 0x01_00, // 1.00
                        iManufacturer: None,
                        iProduct: None,
                        iSerialNumber: None,
                        idProduct: consts::PID,
                        idVendor: consts::VID,
                    };
                    let bytes = desc.bytes();
                    let _ = ep0in.start(&bytes[..core::cmp::min(bytes.len(), length.into())], usbd);
                }
                Descriptor::Configuration { index } => {
                    if index != 0 {
                        dk::usbd::ep0stall(usbd)
                    }

                    let configD = usb2::configuration::Descriptor {
                        bNumInterfaces: core::num::NonZeroU8::new(1).unwrap(),
                        wTotalLength: 18,
                        bConfigurationValue: core::num::NonZeroU8::new(42).unwrap(),
                        iConfiguration: None,
                        bmAttributes: usb2::configuration::bmAttributes {
                            self_powered: true,
                            remote_wakeup: false,
                        },
                        bMaxPower: 250,
                    };

                    let interfaceD = usb2::interface::Descriptor {
                        bInterfaceNumber: 0,
                        bAlternativeSetting: 0,
                        bNumEndpoints: 0,
                        bInterfaceClass: 0,
                        bInterfaceSubClass: 0,
                        bInterfaceProtocol: 0,
                        iInterface: None,
                    };

                    let cD = configD.bytes();
                    let iD = interfaceD.bytes();
                    let sizeD = cD.len() + iD.len();

                    let mut bytes = heapless::Vec::<u8, 1000>::new();
                    bytes.extend_from_slice(&cD).unwrap();
                    bytes.extend_from_slice(&iD).unwrap();

                    let _ = ep0in.start(
                        &bytes[..core::cmp::min(/*bytes.len()*/ sizeD, length.into())],
                        usbd,
                    );
                }

                // TODO implement Configuration descriptor
                // Descriptor::Configuration { .. } => todo!(),

                // stall any other request
                _ => dk::usbd::ep0stall(usbd), /*return Err(())*/
            },
            Request::SetAddress { address } => {
                // On Mac OS you'll get this request before the GET_DESCRIPTOR request so we
                // need to catch it here.

                match *state {
                    State::Default => {
                        if address.is_some() {
                            *state = State::Address(address.unwrap());
                        }
                    }
                    State::Address(x) => {
                        if x == NonZeroU8::new(0).unwrap() {
                            *state = State::Default;
                        } else {
                            *state = State::Address(address.unwrap());
                        }
                    }
                    _ => dk::usbd::ep0stall(usbd),
                }

                // TODO: handle this request properly now.
                //todo!()
            }

            // stall any other request
            _ => dk::usbd::ep0stall(usbd), //return Err(()),
        }

        Ok(())
    }
}
