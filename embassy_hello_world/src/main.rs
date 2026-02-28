//! embassy hello world
//!
//! This is an example of running the embassy executor with multiple tasks
//! concurrently.
//!
//! Including blinky on GPIO2.

#![no_std]
#![no_main]

use core::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_bootloader_esp_idf::partitions::RawPartitionType;
use esp_hal::Config;
use esp_hal::gpio::{Input, InputConfig, Level, Pull, Output, OutputConfig};
use esp_hal::time::Rate;
use esp_hal::peripherals::I2C0;
use esp_hal::i2c::master::{I2c, Config as I2cConfig};
//use esp_hal::timer::Timer;
use esp_hal::timer::timg::TimerGroup;
use esp_println::println;
use esp_println::print;

esp_bootloader_esp_idf::esp_app_desc!();

const HT16K33_ADDR: u8 = 0x70;

#[embassy_executor::task]
async fn run() {
    loop {
        esp_println::println!("Hello world from embassy!");
        Timer::after(Duration::from_millis(1_000)).await;
    }
}

#[embassy_executor::task(pool_size = 2)]
async fn blinky(mut led: Output<'static>) {
    loop {
        led.toggle();
        esp_println::println!("{:?}", led);
        Timer::after(Duration::from_millis(500)).await;
    }
}

#[embassy_executor::task(pool_size=2)]
async fn buttonhandler(mut button: Input<'static>, mode: bool){
    loop {
        button.wait_for_rising_edge().await;
        if mode {
            COUNTER.fetch_add(1, Ordering::Relaxed);
        } else {
            COUNTER.fetch_add(-1, Ordering::Relaxed);
        }
        let current = COUNTER.load(Ordering::Relaxed);
        print!("\r");
        print!("     ");
        print!("\r");
        print!("{}",current);
        Timer::after(Duration::from_millis(300)).await;
    }
}

static COUNTER: AtomicI32 = AtomicI32::new(0);

fn set_pixel(buffer: &mut [u8;16], x:usize, y:usize, on:bool){
    if x < 16 && y < 8 {
        if on {
            buffer[y] |= 1 << x;
        } else {
            buffer[y] &= !(1 << x);
        }
    }
}

fn swap(buffer: &[u8;17]) -> [u8;17]{
    let mut buff = [0u8; 17];
    buff[0] = buffer[0];

    for i in (0..9).step_by(8){
        buff[1+i] = buffer[1+i];
        buff[2+i] = buffer[5+i];
        buff[3+i] = buffer[2+i];
        buff[4+i] = buffer[6+i];

        buff[5+i] = buffer[3+i];
        buff[6+i] = buffer[7+i];
        buff[7+i] = buffer[4+i];
        buff[8+i] = buffer[8+i];
    }
    return buff
}

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    esp_println::println!("Init!");

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(
        timg0.timer0,
        #[cfg(target_arch = "riscv32")]
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT),
    );

    //let led = Output::new(peripherals.GPIO21, Level::High, OutputConfig::default());
    //let led2 = Output::new(peripherals.GPIO2, Level::High, OutputConfig::default());

    let sda = peripherals.GPIO21;
    let scl = peripherals.GPIO22;
    let mut i2c = I2c::new(peripherals.I2C0, I2cConfig::default()).unwrap()
        .with_sda(sda)
        .with_scl(scl);


    //spawner.spawn(blinky(led)).unwrap();
    Timer::after(Duration::from_millis(500)).await;
    //spawner.spawn(blinky(led2)).unwrap();

    i2c.write(HT16K33_ADDR, &[0x21]).unwrap();
    Timer::after_millis(1).await;
    i2c.write(HT16K33_ADDR, &[0x81]).unwrap();
    i2c.write(HT16K33_ADDR, &[0xEF]).unwrap();


    let button_plus = Input::new(peripherals.GPIO34, InputConfig::default().with_pull(Pull::Up));
    spawner.spawn(buttonhandler(button_plus, true)).unwrap();

    let button_min = Input::new(peripherals.GPIO35, InputConfig::default().with_pull(Pull::Up));
    spawner.spawn(buttonhandler(button_min, false)).unwrap();

    let checker = [0x55, 0xAA, 0x55, 0xAA, 0x55, 0xAA, 0x55, 0xAA,
                   0x55, 0xAA, 0x55, 0xAA, 0x55, 0xAA, 0x55, 0xAA,];

    let mut bufferon = [255u8; 16];
    let mut bufferoff = [0u8; 16];

    let mut writebuffer = [0u8; 17];
    for i in 1..bufferon.len()+1 {
        writebuffer[i] = checker[i-1];
    }
    i2c.write(0x70, &swap(&writebuffer)).unwrap();

    let mut num = 0;
    loop {
        Timer::after(Duration::from_millis(500)).await;
        continue;
        num +=1;
        if num == 16{num=0}
        println!("{:?}", writebuffer);

        writebuffer[num] = 255u8;

        i2c.write(0x70, &writebuffer).unwrap();
    }
}
