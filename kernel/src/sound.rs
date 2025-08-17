//! Sound subsystem for RaeenOS
//! Provides basic audio output using PC speaker and future sound card support

use x86_64::instructions::port::Port;
use spin::Mutex;
use lazy_static::lazy_static;
use alloc::string::ToString;

// PC Speaker ports
const PIT_CHANNEL_2: u16 = 0x42;
const PIT_COMMAND: u16 = 0x43;
const SPEAKER_PORT: u16 = 0x61;

// Sound system state
struct SoundSystem {
    enabled: bool,
    current_frequency: u32,
}

lazy_static! {
    static ref SOUND_SYSTEM: Mutex<SoundSystem> = Mutex::new(SoundSystem {
        enabled: false,
        current_frequency: 0,
    });
}

// Initialize the sound system
pub fn init() {
    let mut sound = SOUND_SYSTEM.lock();
    sound.enabled = true;
}

// Play a tone using the PC speaker
pub fn play_tone(frequency: u32, _duration_ms: u32) -> Result<(), ()> {
    if frequency == 0 {
        return stop_sound();
    }
    
    let mut sound = SOUND_SYSTEM.lock();
    if !sound.enabled {
        return Err(());
    }
    
    // Calculate divisor for PIT
    let divisor = 1193180 / frequency;
    if divisor > 65535 {
        return Err(()); // Frequency too low
    }
    
    unsafe {
        // Configure PIT channel 2
        let mut cmd_port = Port::new(PIT_COMMAND);
        let mut data_port = Port::new(PIT_CHANNEL_2);
        let mut speaker_port: Port<u8> = Port::new(SPEAKER_PORT);
        
        // Set PIT to mode 3 (square wave generator)
        cmd_port.write(0xB6u8);
        
        // Send frequency divisor
        data_port.write((divisor & 0xFF) as u8);
        data_port.write((divisor >> 8) as u8);
        
        // Enable speaker
        let speaker_value: u8 = speaker_port.read() | 0x03;
        speaker_port.write(speaker_value);
    }
    
    sound.current_frequency = frequency;
    
    // For duration, we would need a timer callback system
    // For now, just start the tone (caller must stop it)
    Ok(())
}

// Stop the current sound
pub fn stop_sound() -> Result<(), ()> {
    let mut sound = SOUND_SYSTEM.lock();
    
    unsafe {
        let mut speaker_port: Port<u8> = Port::new(SPEAKER_PORT);
        let speaker_value: u8 = speaker_port.read() & 0xFC;
        speaker_port.write(speaker_value);
    }
    
    sound.current_frequency = 0;
    Ok(())
}

// Play predefined system sounds
pub fn play_sound(sound_id: u32, volume: u8, flags: u32) -> Result<(), ()> {
    let _volume = volume; // Volume control not implemented for PC speaker
    let _flags = flags;   // Flags not used yet
    
    match sound_id {
        0 => stop_sound(),                    // Silence
        1 => play_tone(800, 200),            // System beep
        2 => play_tone(1000, 100),           // Success sound
        3 => play_tone(400, 300),            // Error sound
        4 => play_tone(600, 150),            // Warning sound
        5 => play_tone(1200, 50),            // Click sound
        _ => {
            // For unknown sound IDs, play a default beep
            play_tone(800, 100)
        }
    }
}

// Play a sequence of tones (simple melody support)
pub fn play_melody(frequencies: &[u32], durations: &[u32]) -> Result<(), ()> {
    if frequencies.len() != durations.len() {
        return Err(());
    }
    
    // For now, just play the first tone
    // A full implementation would need timer-based sequencing
    if !frequencies.is_empty() {
        play_tone(frequencies[0], durations[0])
    } else {
        Ok(())
    }
}

// Get current sound system status
pub fn get_status() -> (bool, u32) {
    let sound = SOUND_SYSTEM.lock();
    (sound.enabled, sound.current_frequency)
}

/// Process audio buffers for real-time audio thread
/// This function is called by the audio RT thread to handle low-latency audio processing
pub fn process_audio_buffers() {
    let buffer_start = crate::time::get_timestamp_ns();
    
    // For now, this is a placeholder for real audio buffer processing
    // In a full implementation, this would:
    // 1. Check for pending audio data in input/output buffers
    // 2. Process audio effects and mixing
    // 3. Handle audio device I/O
    // 4. Maintain audio timing and synchronization
    
    // Basic audio system maintenance
    let sound = SOUND_SYSTEM.lock();
    if sound.enabled && sound.current_frequency > 0 {
        // Audio is active - perform minimal processing
        // This could include checking for buffer underruns,
        // updating audio device state, etc.
    }
    drop(sound);
    
    // Record audio buffer timing for jitter measurement
    record_audio_buffer_timing(buffer_start);
    
    // For now, just yield to maintain RT thread timing
    // In a real implementation, this would process actual audio buffers
}

/// Audio buffer timing tracking for jitter measurement
static AUDIO_BUFFER_TIMES: spin::Mutex<alloc::collections::VecDeque<u64>> = spin::Mutex::new(alloc::collections::VecDeque::new());
static AUDIO_LAST_BUFFER_TIME: spin::Mutex<Option<u64>> = spin::Mutex::new(None);

/// Record audio buffer timing and calculate jitter
fn record_audio_buffer_timing(buffer_start: u64) {
    let mut last_buffer_time = AUDIO_LAST_BUFFER_TIME.lock();
    
    if let Some(last_time) = *last_buffer_time {
        // Calculate buffer interval (time between buffer starts)
        let buffer_interval = buffer_start - last_time;
        
        // Store buffer intervals for jitter calculation
        let mut buffer_times = AUDIO_BUFFER_TIMES.lock();
        buffer_times.push_back(buffer_interval);
        
        // Keep only the last 100 buffer intervals for measurement
        if buffer_times.len() > 100 {
            buffer_times.pop_front();
        }
        
        // Calculate jitter when we have enough samples
        if buffer_times.len() >= 100 {
            let intervals: alloc::vec::Vec<u64> = buffer_times.iter().copied().collect();
            
            // Audio jitter is the deviation from expected buffer interval
            // For 48kHz with 128 samples, expected interval is ~2.67ms (2670000 ns)
            // For 44.1kHz with 128 samples, expected interval is ~2.9ms (2900000 ns)
            let expected_interval_48khz = 2670000u64; // 2.67ms for 128 samples at 48kHz
            let jitter_values: alloc::vec::Vec<u64> = intervals.iter()
                .map(|&interval| {
                    if interval > expected_interval_48khz {
                        interval - expected_interval_48khz
                    } else {
                        expected_interval_48khz - interval
                    }
                })
                .collect();
            
            // Record audio jitter measurement using SLO system
            let jitter_values_f64: alloc::vec::Vec<f64> = jitter_values.iter()
                .map(|&jitter| (jitter / 1000) as f64) // Convert to microseconds
                .collect();
            
            crate::slo::with_slo_harness(|harness| {
                crate::slo_measure!(harness, 
                    crate::slo::SloCategory::AudioUnderruns, 
                    "audio_buffer_jitter", 
                    "microseconds", 
                    jitter_values_f64.len() as u64, 
                    jitter_values_f64
                );
            });
            
            // Clear the buffer to start fresh measurement
            buffer_times.clear();
        }
    }
    
    *last_buffer_time = Some(buffer_start);
}