use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use x86_64::instructions::port::Port;
use crate::arch::tsc;


static SYSTEM_TIME: AtomicU64 = AtomicU64::new(0);
static TIMER_FREQUENCY: AtomicU64 = AtomicU64::new(1000); // 1000 Hz default
static UPTIME_TICKS: AtomicU64 = AtomicU64::new(0);
static TSC_DEADLINE_ENABLED: AtomicBool = AtomicBool::new(false);

// Real-time clock (RTC) ports
const RTC_SECONDS: u16 = 0x00;
const RTC_MINUTES: u16 = 0x02;
const RTC_HOURS: u16 = 0x04;
const RTC_DAY: u16 = 0x07;
const RTC_MONTH: u16 = 0x08;
const RTC_YEAR: u16 = 0x09;
const RTC_STATUS_A: u16 = 0x0A;
const RTC_STATUS_B: u16 = 0x0B;

const CMOS_ADDRESS: u16 = 0x70;
const CMOS_DATA: u16 = 0x71;

#[derive(Debug, Clone, Copy)]
pub struct DateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl DateTime {
    pub fn to_timestamp(&self) -> u64 {
        // Simple timestamp calculation (days since epoch * seconds per day + time)
        let days_since_epoch = days_since_epoch(self.year, self.month, self.day);
        let seconds_today = (self.hour as u64 * 3600) + (self.minute as u64 * 60) + (self.second as u64);
        (days_since_epoch * 86400) + seconds_today
    }
}

fn days_since_epoch(year: u16, month: u8, day: u8) -> u64 {
    // Simple calculation from Unix epoch (1970-01-01)
    let mut days = 0u64;
    
    // Add days for complete years
    for y in 1970..year {
        if is_leap_year(y) {
            days += 366;
        } else {
            days += 365;
        }
    }
    
    // Add days for complete months in current year
    let days_in_month = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for m in 1..month {
        days += days_in_month[(m - 1) as usize] as u64;
        if m == 2 && is_leap_year(year) {
            days += 1; // Leap day
        }
    }
    
    // Add remaining days
    days + (day - 1) as u64
}

fn is_leap_year(year: u16) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn bcd_to_binary(bcd: u8) -> u8 {
    (bcd & 0x0F) + ((bcd >> 4) * 10)
}

fn read_rtc_register(reg: u16) -> u8 {
    unsafe {
        let mut addr_port = Port::new(CMOS_ADDRESS);
        let mut data_port = Port::new(CMOS_DATA);
        
        addr_port.write(reg as u8);
        data_port.read()
    }
}

fn is_rtc_updating() -> bool {
    read_rtc_register(RTC_STATUS_A) & 0x80 != 0
}

pub fn read_rtc() -> DateTime {
    // Wait for RTC to not be updating
    while is_rtc_updating() {
        core::hint::spin_loop();
    }
    
    let second = read_rtc_register(RTC_SECONDS);
    let minute = read_rtc_register(RTC_MINUTES);
    let hour = read_rtc_register(RTC_HOURS);
    let day = read_rtc_register(RTC_DAY);
    let month = read_rtc_register(RTC_MONTH);
    let year = read_rtc_register(RTC_YEAR);
    
    // Check if RTC is in BCD mode
    let status_b = read_rtc_register(RTC_STATUS_B);
    let is_bcd = (status_b & 0x04) == 0;
    
    DateTime {
        year: if is_bcd { bcd_to_binary(year) as u16 + 2000 } else { year as u16 + 2000 },
        month: if is_bcd { bcd_to_binary(month) } else { month },
        day: if is_bcd { bcd_to_binary(day) } else { day },
        hour: if is_bcd { bcd_to_binary(hour) } else { hour },
        minute: if is_bcd { bcd_to_binary(minute) } else { minute },
        second: if is_bcd { bcd_to_binary(second) } else { second },
    }
}

pub fn init() {
    // Read initial time from RTC
    let rtc_time = read_rtc();
    let timestamp = rtc_time.to_timestamp();
    SYSTEM_TIME.store(timestamp, Ordering::SeqCst);
    
    // Initialize TSC subsystem first
    if let Ok(()) = tsc::init() {
        crate::serial::_print(format_args!("[Timer] TSC initialized with frequency: {} Hz\n", tsc::get_frequency()));
        
        // Try to enable TSC-deadline timer if supported
        if crate::arch::has_cpu_feature(crate::arch::CpuFeature::TscDeadline) {
            init_tsc_deadline_timer();
        } else {
            crate::serial::_print(format_args!("[Timer] TSC-deadline not supported, falling back to PIT\n"));
            init_pit();
        }
    } else {
        crate::serial::_print(format_args!("[Timer] TSC initialization failed, using PIT\n"));
        init_pit();
        // Fallback TSC calibration for performance counters
        calibrate_tsc_fallback();
    }
}

/// Initialize timer with APIC support
pub fn init_with_apic() {
    // Read initial time from RTC
    let rtc_time = read_rtc();
    let timestamp = rtc_time.to_timestamp();
    SYSTEM_TIME.store(timestamp, Ordering::SeqCst);
    
    // Initialize TSC subsystem first
    if let Ok(()) = tsc::init() {
        crate::serial::_print(format_args!("[Timer] TSC initialized with frequency: {} Hz\n", tsc::get_frequency()));
        
        // Enable TSC-deadline timer if supported
        if crate::arch::has_cpu_feature(crate::arch::CpuFeature::TscDeadline) {
            init_tsc_deadline_timer();
        } else {
            crate::serial::_print(format_args!("[Timer] TSC-deadline not supported, falling back to PIT\n"));
            init_pit();
        }
    } else {
        crate::serial::_print(format_args!("[Timer] TSC initialization failed, using PIT\n"));
        init_pit();
        // Fallback TSC calibration for performance counters
        calibrate_tsc_fallback();
    }
}

fn init_pit() {
    const PIT_FREQUENCY: u32 = 1193182; // PIT base frequency
    const TARGET_FREQUENCY: u32 = 1000; // 1000 Hz (1ms intervals)
    
    let divisor = PIT_FREQUENCY / TARGET_FREQUENCY;
    
    unsafe {
        let mut command_port = Port::new(0x43);
        let mut data_port = Port::new(0x40);
        
        // Configure PIT channel 0
        command_port.write(0x36u8); // Channel 0, lobyte/hibyte, rate generator
        
        // Set divisor
        data_port.write((divisor & 0xFF) as u8);
        data_port.write((divisor >> 8) as u8);
    }
    
    TIMER_FREQUENCY.store(TARGET_FREQUENCY as u64, Ordering::SeqCst);
}

/// Initialize TSC-deadline timer
fn init_tsc_deadline_timer() {
    const TARGET_FREQUENCY: u64 = 1000; // 1000 Hz (1ms intervals)
    
    if !tsc::is_invariant_available() {
        crate::serial::_print(format_args!("[Timer] Invariant TSC not available, falling back to PIT\n"));
        init_pit();
        return;
    }
    
    let tsc_freq = tsc::get_frequency();
    if tsc_freq > 0 {
        let deadline_interval = tsc_freq / TARGET_FREQUENCY;
        
        // Set initial deadline
        let current_tsc = tsc::read_tsc();
        let deadline = current_tsc + deadline_interval;
        tsc::deadline::set_deadline(deadline);
        
        TSC_DEADLINE_ENABLED.store(true, Ordering::SeqCst);
        TIMER_FREQUENCY.store(TARGET_FREQUENCY, Ordering::SeqCst);
        crate::serial::_print(format_args!("[Timer] TSC-deadline timer initialized at {} Hz\n", TARGET_FREQUENCY));
    } else {
        crate::serial::_print(format_args!("[Timer] TSC not calibrated, falling back to PIT\n"));
        init_pit();
    }
}

/// Initialize APIC timer in one-shot mode
/// TODO: Re-enable once APIC module is available
#[allow(dead_code)]
fn init_apic_timer() {
    // const TARGET_FREQUENCY: u64 = 1000; // 1000 Hz (1ms intervals)
    // 
    // // Configure APIC timer in one-shot mode
    // if let Err(e) = crate::apic::set_timer_mode(crate::apic::TimerMode::OneShot) {
    //     crate::serial::_print(format_args!("[Timer] Failed to set APIC timer mode: {}\n", e));
    //     // Fall back to PIT
    //     init_pit();
    //     return;
    // }
    // 
    // // Set timer divisor and initial count
    // let divisor = 16; // Divide by 16
    // let initial_count = 1000000; // Approximate 1ms at typical APIC frequencies
    // 
    // crate::apic::set_timer_divisor(divisor);
    // crate::apic::set_timer_initial_count(initial_count);
    // 
    // TIMER_FREQUENCY.store(TARGET_FREQUENCY, Ordering::SeqCst);
    // crate::serial::_print(format_args!("[Timer] APIC timer initialized at {} Hz\n", TARGET_FREQUENCY));
    
    // Placeholder implementation - fall back to PIT
    init_pit();
}

// Called by timer interrupt handler
pub fn tick() {
    let ticks = UPTIME_TICKS.fetch_add(1, Ordering::SeqCst) + 1;
    let frequency = TIMER_FREQUENCY.load(Ordering::SeqCst);
    
    // Update system time every second
    if ticks % frequency == 0 {
        SYSTEM_TIME.fetch_add(1, Ordering::SeqCst);
    }
    
    // Handle TSC-deadline timer if enabled (tickless mode)
    if TSC_DEADLINE_ENABLED.load(Ordering::SeqCst) {
        schedule_next_timer_deadline();
    }
}

/// Schedule the next timer deadline based on scheduler requirements (tickless)
pub fn schedule_next_timer_deadline() {
    if !TSC_DEADLINE_ENABLED.load(Ordering::SeqCst) {
        return;
    }
    
    let tsc_freq = tsc::get_frequency();
    if tsc_freq == 0 {
        return;
    }
    
    // Get the next deadline from the scheduler
    let next_deadline_us = crate::process::get_next_scheduler_deadline_us();
    
    if next_deadline_us > 0 {
        // Set TSC deadline timer for the calculated deadline
        tsc::deadline::set_deadline_us(next_deadline_us);
    } else {
        // No specific deadline needed, use default 1ms interval
        tsc::deadline::set_deadline_us(1000);
    }
}

/// Set a one-shot timer for a specific deadline in microseconds
pub fn set_timer_deadline_us(us: u64) {
    if TSC_DEADLINE_ENABLED.load(Ordering::SeqCst) {
        tsc::deadline::set_deadline_us(us);
    }
}

/// Clear any pending timer deadline
pub fn clear_timer_deadline() {
    if TSC_DEADLINE_ENABLED.load(Ordering::SeqCst) {
        tsc::deadline::clear_deadline();
    }
}

pub fn get_timestamp() -> u64 {
    SYSTEM_TIME.load(Ordering::SeqCst)
}

pub fn get_uptime_ms() -> u64 {
    let ticks = UPTIME_TICKS.load(Ordering::SeqCst);
    let frequency = TIMER_FREQUENCY.load(Ordering::SeqCst);
    (ticks * 1000) / frequency
}

pub fn get_uptime_seconds() -> u64 {
    get_uptime_ms() / 1000
}

pub fn get_system_uptime() -> u64 {
    get_uptime_seconds()
}

pub fn get_datetime() -> DateTime {
    // For now, just read from RTC each time
    // In a real implementation, we'd maintain this in memory
    read_rtc()
}

pub fn sleep_ms(milliseconds: u64) {
    let start = get_uptime_ms();
    while get_uptime_ms() - start < milliseconds {
        core::hint::spin_loop();
    }
}

pub fn sleep_seconds(seconds: u64) {
    sleep_ms(seconds * 1000);
}

// High-precision timer using TSC (Time Stamp Counter)
// Legacy TSC frequency for fallback calibration
static TSC_FREQUENCY_FALLBACK: AtomicU64 = AtomicU64::new(0);

/// Fallback TSC calibration for performance counters when arch TSC init fails
pub fn calibrate_tsc_fallback() {
    // Calibrate TSC frequency using PIT
    let start_tsc = tsc::read_tsc();
    let start_time = get_uptime_ms();
    
    // Wait for 100ms
    sleep_ms(100);
    
    let end_tsc = tsc::read_tsc();
    let end_time = get_uptime_ms();
    
    let elapsed_ms = end_time - start_time;
    let tsc_diff = end_tsc - start_tsc;
    
    if elapsed_ms > 0 {
        let tsc_freq = (tsc_diff * 1000) / elapsed_ms;
        TSC_FREQUENCY_FALLBACK.store(tsc_freq, Ordering::SeqCst);
        crate::serial::_print(format_args!("[Timer] Fallback TSC calibrated at {} Hz\n", tsc_freq));
    }
}

/// Get TSC frequency (prefer arch module, fallback to local calibration)
fn get_tsc_frequency() -> u64 {
    let arch_freq = tsc::get_frequency();
    if arch_freq > 0 {
        arch_freq
    } else {
        TSC_FREQUENCY_FALLBACK.load(Ordering::SeqCst)
    }
}

pub fn get_precise_time_ns() -> u64 {
    if tsc::is_invariant_available() {
        // Use arch module TSC functions for best precision
        let current_tsc = tsc::read_tsc();
        tsc::ticks_to_ns(current_tsc)
    } else {
        // Fallback to local TSC calibration
        let current_tsc = tsc::read_tsc();
        let freq = get_tsc_frequency();
        
        if freq > 0 {
            (current_tsc * 1_000_000_000) / freq
        } else {
            // Fallback to millisecond precision
            get_uptime_ms() * 1_000_000
        }
    }
}

/// Get timestamp in nanoseconds for SLO measurements
pub fn get_timestamp_ns() -> u64 {
    get_precise_time_ns()
}

// Performance measurement utilities
pub struct PerformanceCounter {
    start_tsc: u64,
    start_time: u64,
}

impl PerformanceCounter {
    pub fn new() -> Self {
        Self {
            start_tsc: tsc::read_tsc(),
            start_time: get_uptime_ms(),
        }
    }
    
    pub fn elapsed_ns(&self) -> u64 {
        let current_tsc = tsc::read_tsc();
        
        if tsc::is_invariant_available() {
            let tsc_diff = current_tsc - self.start_tsc;
            tsc::ticks_to_ns(tsc_diff)
        } else {
            let freq = get_tsc_frequency();
            
            if freq > 0 {
                let tsc_diff = current_tsc - self.start_tsc;
                (tsc_diff * 1_000_000_000) / freq
            } else {
                let time_diff = get_uptime_ms() - self.start_time;
                time_diff * 1_000_000
            }
        }
    }
    
    pub fn elapsed_us(&self) -> u64 {
        self.elapsed_ns() / 1000
    }
    
    pub fn elapsed_ms(&self) -> u64 {
        self.elapsed_ns() / 1_000_000
    }
}

// Gaming mode optimizations
static GAMING_MODE: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

pub fn set_gaming_mode(enabled: bool) {
    GAMING_MODE.store(enabled, Ordering::SeqCst);
    
    if enabled {
        // Increase timer frequency for better precision
        TIMER_FREQUENCY.store(2000, Ordering::SeqCst); // 2000 Hz
        init_pit();
    } else {
        // Reset to normal frequency
        TIMER_FREQUENCY.store(1000, Ordering::SeqCst); // 1000 Hz
        init_pit();
    }
}

pub fn is_gaming_mode() -> bool {
    GAMING_MODE.load(Ordering::SeqCst)
}

// Frame timing utilities for gaming
pub struct FrameTimer {
    last_frame: u64,
    target_fps: u64,
    frame_count: u64,
    fps_start_time: u64,
    current_fps: f64,
}

impl FrameTimer {
    pub fn new(target_fps: u64) -> Self {
        let now = get_precise_time_ns();
        Self {
            last_frame: now,
            target_fps,
            frame_count: 0,
            fps_start_time: now,
            current_fps: 0.0,
        }
    }
    
    pub fn wait_for_next_frame(&mut self) {
        let target_frame_time = 1_000_000_000 / self.target_fps; // nanoseconds
        let now = get_precise_time_ns();
        let elapsed = now - self.last_frame;
        
        if elapsed < target_frame_time {
            let sleep_time = target_frame_time - elapsed;
            // Convert to milliseconds for sleep
        let sleep_ms_val = sleep_time / 1_000_000;
        if sleep_ms_val > 0 {
            sleep_ms(sleep_ms_val);
            }
        }
        
        self.last_frame = get_precise_time_ns();
        self.frame_count += 1;
        
        // Update FPS calculation every second
        if now - self.fps_start_time >= 1_000_000_000 {
            self.current_fps = self.frame_count as f64;
            self.frame_count = 0;
            self.fps_start_time = now;
        }
    }
    
    pub fn get_fps(&self) -> f64 {
        self.current_fps
    }
    
    pub fn set_target_fps(&mut self, fps: u64) {
        self.target_fps = fps;
    }
}


