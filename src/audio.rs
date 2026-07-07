use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug)]
pub struct TiaAudioState {
    pub audc0: u8,
    pub audf0: u8,
    pub audv0: u8,
    pub audc1: u8,
    pub audf1: u8,
    pub audv1: u8,
    /// Quando true, força um beep contínuo para validar o backend de áudio.
    pub test_beep: bool,
}

impl Default for TiaAudioState {
    fn default() -> Self {
        Self { audc0: 0, audf0: 0, audv0: 0, audc1: 0, audf1: 0, audv1: 0, test_beep: false }
    }
}

pub type SharedAudioState = Arc<Mutex<TiaAudioState>>;

pub fn shared_audio_state() -> SharedAudioState {
    Arc::new(Mutex::new(TiaAudioState::default()))
}

pub fn set_test_beep(state: &SharedAudioState, enabled: bool) {
    if let Ok(mut s) = state.lock() {
        s.test_beep = enabled;
        if enabled {
            s.audc0 = 0x04;
            s.audf0 = 18;
            s.audv0 = 12;
            s.audc1 = 0;
            s.audf1 = 0;
            s.audv1 = 0;
        }
    }
}

#[cfg(feature = "audio")]
mod backend {
    use super::{SharedAudioState, TiaAudioState};
    use std::{f32::consts::TAU, time::Duration};

    pub struct AudioEngine {
        _stream: rodio::OutputStream,
        _sink: rodio::Sink,
    }

    impl AudioEngine {
        pub fn new(state: SharedAudioState) -> anyhow::Result<Self> {
            let (stream, handle) = rodio::OutputStream::try_default()?;
            let sink = rodio::Sink::try_new(&handle)?;
            sink.append(TiaAudioSource::new(state));
            sink.play();
            log::info!("dispositivo de áudio aberto via rodio/cpal");
            Ok(Self { _stream: stream, _sink: sink })
        }
    }

    struct TiaAudioSource {
        state: SharedAudioState,
        sample_rate: u32,
        phase0: f32,
        phase1: f32,
        noise0: u32,
        noise1: u32,
        last0: f32,
        last1: f32,
    }

    impl TiaAudioSource {
        fn new(state: SharedAudioState) -> Self {
            Self { state, sample_rate: 44_100, phase0: 0.0, phase1: 0.0, noise0: 0xACE1, noise1: 0xBEEF, last0: 0.0, last1: 0.0 }
        }

        fn chan_sample(phase: &mut f32, noise: &mut u32, last: &mut f32, sample_rate: u32, audc: u8, audf: u8, audv: u8) -> f32 {
            let vol = (audv & 0x0f) as f32 / 15.0;
            if vol <= 0.0 { *last = 0.0; return 0.0; }

            // Aproximação do clock TIA. O ponto importante aqui é NÃO atualizar
            // ruído a cada sample de áudio, senão vira só chiado branco.
            let div = audf as f32 + 1.0;
            let mut freq = 15_700.0 / div;
            if matches!(audc & 0x0f, 0x06 | 0x0a) { freq *= 0.5; }
            freq = freq.clamp(30.0, 5_500.0);

            *phase += freq / sample_rate as f32;
            let wrapped = *phase >= 1.0;
            if wrapped { *phase -= 1.0; }

            let waveform = match audc & 0x0f {
                // Set-to-1 / volume only: use DC leve, evitando estalo exagerado.
                0x00 | 0x0b => 0.35,
                // Polinômios de ruído: sample-and-hold no tick do canal.
                0x01 | 0x02 | 0x03 | 0x08 | 0x09 => {
                    if wrapped {
                        let bit = ((*noise) ^ (*noise >> 1) ^ (*noise >> 21) ^ (*noise >> 31)) & 1;
                        *noise = (*noise >> 1) | (bit << 31);
                        *last = if (*noise & 1) != 0 { 1.0 } else { -1.0 };
                    }
                    *last
                }
                // Divisores tonais/quadrados.
                0x04 | 0x05 | 0x0c | 0x0d => if *phase < 0.5 { 1.0 } else { -1.0 },
                // Tons mais suaves para reduzir aspereza no motor do Enduro.
                0x06 | 0x0a => (TAU * *phase).sin(),
                _ => if *phase < 0.5 { 1.0 } else { -1.0 },
            };

            // Ganho conservador para não saturar quando os dois canais estão ativos.
            waveform * vol * 0.10
        }
    }

    impl Iterator for TiaAudioSource {
        type Item = f32;
        fn next(&mut self) -> Option<f32> {
            let s: TiaAudioState = self.state.lock().map(|g| *g).unwrap_or_default();
            if s.test_beep {
                self.phase0 += 440.0 / self.sample_rate as f32;
                if self.phase0 >= 1.0 { self.phase0 -= 1.0; }
                return Some(if self.phase0 < 0.5 { 0.25 } else { -0.25 });
            }
            let a = Self::chan_sample(&mut self.phase0, &mut self.noise0, &mut self.last0, self.sample_rate, s.audc0, s.audf0, s.audv0);
            let b = Self::chan_sample(&mut self.phase1, &mut self.noise1, &mut self.last1, self.sample_rate, s.audc1, s.audf1, s.audv1);
            Some((a + b).clamp(-1.0, 1.0))
        }
    }

    impl rodio::Source for TiaAudioSource {
        fn current_frame_len(&self) -> Option<usize> { None }
        fn channels(&self) -> u16 { 1 }
        fn sample_rate(&self) -> u32 { self.sample_rate }
        fn total_duration(&self) -> Option<Duration> { None }
    }
}

#[cfg(not(feature = "audio"))]
mod backend {
    use super::SharedAudioState;

    pub struct AudioEngine;

    impl AudioEngine {
        pub fn new(_state: SharedAudioState) -> anyhow::Result<Self> {
            anyhow::bail!("áudio compilado sem backend. Rode com `--features audio` e instale libasound2-dev/pkg-config no Linux");
        }
    }
}

pub use backend::AudioEngine;
