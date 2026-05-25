#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]


use defmt::info;
use esp_backtrace as _;

use core::cell::{Cell, RefCell};

use critical_section::Mutex;
use esp_backtrace as _;
use esp_println::println;

use esp_hal::{
    Config, clock::CpuClock, delay::Delay, gpio::{Event, Input, InputConfig, Io, Level, Output, OutputConfig, Pull}, handler, main, peripherals::SYSTIMER, timer::systimer::SystemTimer
};
use esp_hal::timer::systimer;
use embedded_hal::digital::OutputPin;
// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
/*G_BUTTON contient le bouton pour que le handler d’interruption puisse y accéder plus tard. 
Le Option est là parce que le bouton est initialisé dans main, puis placé dans la variable globale ensuite. 
Input<'static> signifie que le driver est conservé pour toute la durée du programme. 
G_DELAYMS contient la valeur du délai courant, initialisée à 1000 ms, que l’interruption va modifier.
Le besoin d’un Input global vient du fait que le handler GPIO doit pouvoir appeler clear_interrupt() sur le même driver. 
La doc esp-hal dit bien que Input::new crée un driver d’entrée et que Io peut servir à configurer le handler d’interruptions GPIO. */
static G_BUTTON: Mutex<RefCell<Option<Input<'static>>>> = Mutex::new(RefCell::new(None));
static G_DELAY: Mutex<Cell<u32>> = Mutex::new(Cell::new(2000));
struct Led <P> {
    pin: P,
    active: bool,
}
impl <P>Led<P>
where P : OutputPin, {
    pub fn new(pin: P, active: bool ) -> Self {
        Led{
            pin,
            active
        }
    }
    pub fn on (&mut self) {
        if self.active {
            let _ = self.pin.set_high();
        }else {
            let _ = self.pin.set_low();
        }
    }
    pub fn off (&mut self) {
        if self.active {
            let _ = self.pin.set_low();       
        }
        else {
            let _ = self.pin.set_high();
        }
    }
    }

#[main]
fn main() -> ! {
    // generator version: 1.3.0
    // generator parameters: --chip esp32s3 -o unstable-hal -o defmt -o esp-backtrace -o vscode

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let _peripherals = esp_hal::init(config);

    let sys_t = SystemTimer::new(_peripherals.SYSTIMER);
    let alarm = sys_t.alarm0;

    /// HAL prépare la couche matérielle de multiplexage des broches. 
    /// Ensuite, quand tu crées Input ou Output, le HAL peut configurer la broche choisie 
    /// correctement à travers cette infrastructure
    let mut io = Io::new(_peripherals.IO_MUX);
    io.set_interrupt_handler(hanfler_gp);
    
    let  led_r = Output::new(_peripherals.GPIO4, Level::Low, OutputConfig::default());
    let  led_g = Output::new(_peripherals.GPIO5, Level::Low, OutputConfig::default());
    let mut pi_r = Led::new(led_r,false); //but.is_high() could be replaced by false 
    //let mut pi_g = Led::new(led_g, but.is_high()); // but.is_high() could be replaced by false

    let mut but = Input::new(_peripherals.GPIO6, InputConfig::default().with_pull(Pull::Up));
    ///arme une interruption sur front descendant
    but.listen(Event::FallingEdge);

    critical_section::with(|cs|{
        G_BUTTON.borrow_ref_mut(cs).replace(but);
    });

    let delay = Delay::new();

    loop {
        info!("led blinking!");
        
     
        pi_r.on();
      
        delay.delay_millis(critical_section::with(|cs| G_DELAY.borrow(cs).get()));
      
        pi_r.off();
       
        delay.delay_millis(critical_section::with(|cs| G_DELAY.borrow(cs).get()));
        }
          

    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.1.0/examples

#[handler]
fn hanfler_gp ( ){
    let mut cpt =0;
    info!("Button pressed to get led activated");

    critical_section::with(|cs| {
        let current = G_DELAY.borrow(cs).get();
        let next = if current <= 500 {2000} else {
            current - 500
        };
        G_DELAY.borrow(cs).set(next);

        if let Some(button) = G_BUTTON.borrow_ref_mut(cs).as_mut() {
            button.clear_interrupt();
            cpt +=1 ;
            info!("print le compteur: {}",cpt);
        }
        
    })

}