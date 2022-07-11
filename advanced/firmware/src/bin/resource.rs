#![no_main]
#![no_std]

// this imports `beginner/apps/lib.rs` to retrieve our global logger + panicking-behavior
use firmware as _;

#[rtic::app(device = dk, peripherals = false)]
mod app {
    use cortex_m::asm;
    use dk::peripheral::POWER;

    #[local]
    struct MyLocalResources {
        power: POWER,
        counter: u32,
    }

    #[shared]
    struct MySharedResources {}

    #[init]
    fn init(_cx: init::Context) -> (MySharedResources, MyLocalResources, init::Monotonics) {
        let board = dk::init().unwrap();

        let power = board.power;

        power.intenset.write(|w| w.usbdetected().set_bit());

        defmt::println!("USBDETECTED interrupt enabled");

        (
            MySharedResources {},
            MyLocalResources { power, counter: 0 },
            init::Monotonics(),
        )
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        loop {
            defmt::println!("idle: going to sleep");
            asm::wfi();
            defmt::println!("idle: woke up");
        }
    }

    #[task(binds = POWER_CLOCK, local = [power, counter])]
    //                                      ^^^^^^^ resource access list
    fn on_power_event(cx: on_power_event::Context) {
        // resources available to this task
        let resources = cx.local;
        let counter = resources.counter;
        *counter = *counter + 1;

        defmt::println!("POWER event occurred {:?}", *counter);

        // the POWER peripheral can be accessed through a reference
        let power: &mut POWER = resources.power;

        // clear the interrupt flag; otherwise this task will run again after it returns
        power.events_usbdetected.reset();
    }
}
