use anyhow::Result;
use clap::Parser;
use pixels::{Pixels, SurfaceTexture};
use std::{fs, path::PathBuf, time::{Duration, Instant}};
use winit::{dpi::LogicalSize, event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent}, event_loop::{ControlFlow, EventLoop}, window::WindowBuilder};

mod cpu;
mod bus;
mod tia;
mod riot;
mod cartridge;
mod input;
mod console;
mod audio;

use console::Atari2600;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Caminho da ROM .bin do Atari 2600
    rom: PathBuf,
    /// Escala da janela
    #[arg(short, long, default_value_t = 3)]
    scale: u32,
    /// Clock aproximado. Use 1 para normal, >1 para turbo.
    #[arg(short = 't', long, default_value_t = 1)]
    speed: u32,
    /// Liga trace detalhado da CPU/TIA. Use com RUST_LOG=trace.
    #[arg(long, default_value_t = false)]
    trace: bool,
    /// Primeiro color clock visível. Ajuste fino para ROMs sensíveis como Enduro.
    #[arg(long, default_value_t = 68)]
    visible_start: usize,
    /// Deslocamento horizontal final em pixels. Valor negativo move para esquerda.
    #[arg(long, default_value_t = -4)]
    x_adjust: isize,
    /// Corta algumas scanlines do início da área visível.
    #[arg(long, default_value_t = 0)]
    y_crop: usize,
    /// Desliga áudio.
    #[arg(long, default_value_t = false)]
    no_audio: bool,
    /// Força um beep contínuo para testar se o backend de áudio está funcionando.
    #[arg(long, default_value_t = false)]
    audio_test: bool,
    /// FPS alvo. NTSC geralmente é próximo de 60.
    #[arg(long, default_value_t = 60)]
    fps: u32,
}

fn main() -> Result<()> {
    let mut logger = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    );
    // Mesmo com RUST_LOG=debug, não deixe o backend gráfico poluir o terminal.
    logger
        .filter_module("wgpu_core", log::LevelFilter::Warn)
        .filter_module("wgpu_hal", log::LevelFilter::Warn)
        .filter_module("wgpu", log::LevelFilter::Warn)
        .filter_module("naga", log::LevelFilter::Warn)
        .init();
    let args = Args::parse();
    let rom = fs::read(&args.rom)?;
    let mut atari = Atari2600::new(rom)?;
    atari.set_video_calibration(args.visible_start, args.x_adjust, args.y_crop);
    atari.trace = args.trace;
    if args.trace { log::warn!("Trace ligado: a saída será MUITO grande. Use: RUST_LOG=trace cargo run -- <rom> --trace"); }
    if args.audio_test {
        audio::set_test_beep(&atari.audio_state, true);
        log::warn!("--audio-test ligado: deve sair um beep contínuo. Se não sair, o problema é no device/mixer do sistema.");
    }
    let _audio_engine = if args.no_audio {
        log::info!("áudio desligado (--no-audio)");
        None
    } else {
        match audio::AudioEngine::new(atari.audio_state.clone()) {
            Ok(engine) => { log::info!("áudio TIA básico ligado"); Some(engine) },
            Err(err) => { log::warn!("não consegui iniciar áudio: {err}"); None },
        }
    };

    let event_loop = EventLoop::new();
    let width = tia::VISIBLE_WIDTH as u32;
    let height = tia::VISIBLE_HEIGHT as u32;
    let window = WindowBuilder::new()
        .with_title("Atari 2600 Rust Emulator")
        .with_inner_size(LogicalSize::new(width * args.scale, height * args.scale))
        .with_min_inner_size(LogicalSize::new(width, height))
        .build(&event_loop)?;

    let surface_texture = SurfaceTexture::new(width * args.scale, height * args.scale, &window);
    let mut pixels = Pixels::new(width, height, surface_texture)?;
    let mut last = Instant::now();
    let frame_time = Duration::from_secs_f64(1.0 / args.fps.max(1) as f64);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(last + frame_time);
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput { input, .. } => handle_key(input, &mut atari),
                _ => {}
            },
            Event::MainEventsCleared => {
                if last.elapsed() >= frame_time {
                    for _ in 0..args.speed { atari.run_frame(); }
                    window.request_redraw();
                    last = Instant::now();
                }
            }
            Event::RedrawRequested(_) => {
                pixels.frame_mut().copy_from_slice(atari.framebuffer());
                if pixels.render().is_err() { *control_flow = ControlFlow::Exit; }
            }
            _ => {}
        }
    });
}

fn handle_key(input: KeyboardInput, atari: &mut Atari2600) {
    let pressed = input.state == ElementState::Pressed;
    if let Some(key) = input.virtual_keycode {
        match key {
            VirtualKeyCode::Left => atari.input.set_left(pressed),
            VirtualKeyCode::Right => atari.input.set_right(pressed),
            VirtualKeyCode::Up => atari.input.set_up(pressed),
            VirtualKeyCode::Down => atari.input.set_down(pressed),
            VirtualKeyCode::Space | VirtualKeyCode::Z | VirtualKeyCode::X => atari.input.set_fire(pressed),
            VirtualKeyCode::Return => atari.input.set_reset_switch(pressed),
            VirtualKeyCode::F1 | VirtualKeyCode::Key1 => atari.input.set_select(pressed),
            VirtualKeyCode::F2 | VirtualKeyCode::Key2 => atari.input.set_reset_switch(pressed),
            VirtualKeyCode::R if pressed => atari.reset(),
            VirtualKeyCode::C if pressed => atari.input.toggle_color(),
            VirtualKeyCode::A if pressed => atari.input.toggle_p0_difficulty(),
            VirtualKeyCode::S if pressed => atari.input.toggle_p1_difficulty(),
            _ => {}
        }
    }
}
