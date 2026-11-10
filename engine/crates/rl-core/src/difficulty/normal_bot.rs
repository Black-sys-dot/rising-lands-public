// Appended to an instrumented copy of the existing play_bot in motor/target.
// Every decision, action, phase and stop still uses that bot's existing run().
static NORMAL_ENABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
thread_local! {
    static OBSERVATIONS: std::cell::RefCell<Vec<(u64,u32,bool,String)>> = const { std::cell::RefCell::new(Vec::new()) };
}
fn observe_world(world: &World, renderer: &Renderer, input: FrameInput, seed: u64, frame: u32, final_state: bool) -> AnyResult<()> {
    if !final_state && frame % 500 != 0 { return Ok(()); }
    let m = world.project()?;
    let mut bytes = Vec::new();
    for (address, block) in m.blocks() {
        bytes.extend_from_slice(&address.to_le_bytes());
        bytes.extend_from_slice(&(block.len() as u32).to_le_bytes());
        bytes.extend_from_slice(block);
    }
    bytes.extend_from_slice(&renderer.surface);
    bytes.extend_from_slice(&renderer.cache.keys);
    bytes.extend_from_slice(&renderer.cache.raw);
    bytes.extend_from_slice(&(renderer.cache.cursor as u64).to_le_bytes());
    bytes.extend_from_slice(&renderer.cache_generation.to_le_bytes());
    bytes.extend_from_slice(format!("{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}", input, world.generation, world.frame_phase, world.clock, world.frame_events, world.frame_freed, world.player_events, world.player_cursor, world.state()).as_bytes());
    let digest: String=sha256(&bytes).iter().map(|v|format!("{v:02x}")).collect();
    OBSERVATIONS.with(|rows|rows.borrow_mut().push((seed,frame,final_state,digest)));
    Ok(())
}
fn observations() -> Vec<(u64,u32,bool,String)> {
    OBSERVATIONS.with(|rows|std::mem::take(&mut *rows.borrow_mut()))
}
fn main() -> AnyResult<()> {
    let args: Vec<_>=env::args().skip(1).collect();
    if args.len()!=3 { return Err("usage: difficulty_normal INPUTS CD_ROOT OUTPUT_JSONL".into()); }
    let start=Start::load(Path::new(&args[0]),Path::new(&args[1]))?;
    // Exercise both non-Normal presets on the actual original stage, too.
    for preset in [rl_core::difficulty::Preset::Easy,rl_core::difficulty::Preset::Hard] {
        let mut world=rl_core::difficulty::load_fresh(start.memory.clone(),TickClock::new(20,0)?)?;
        let before=world.project()?;
        rl_core::difficulty::apply(&mut world,preset)?;
        let after=world.project()?;
        assert_eq!(world.state().world_rng.as_ref(),World::load(before.clone(),TickClock::new(20,0)?)?.state().world_rng.as_ref());
        assert_eq!(after.u32(0x50a9c0)?,if preset==rl_core::difficulty::Preset::Easy {2}else{10});
        assert_ne!(before,after);
        println!("actual S0 {preset:?}: first raid period={}, mask={}, current={}, weights={}/{}/{}, local resources={}/{}/{}",after.u16(0x50a9d4)?,after.u16(0x50a9d2)?,after.u8(0x50a9d6)?,after.u32(0x50a9c0)?,after.u32(0x50a9c4)?,after.u32(0x50a9c8)?,after.u16(0x4e2ea8)?,after.u16(0x4e2eb0)?,after.u16(0x4e2eb8)?);
        world.frame_step(1,start.input)?;
    }
    let mut output=fs::OpenOptions::new().write(true).create_new(true).open(&args[2])?;
    for seed in 1..=20 {
        NORMAL_ENABLED.store(false,std::sync::atomic::Ordering::Relaxed);
        let baseline=run(&start,seed,36000,32,None,None)?;
        let expected=observations();
        NORMAL_ENABLED.store(true,std::sync::atomic::Ordering::Relaxed);
        let normal=run(&start,seed,36000,32,None,None)?;
        let actual=observations();
        assert_eq!(expected,actual,"Normal state mismatch for seed {seed}");
        assert_eq!(baseline.json(),normal.json(),"Normal bot result mismatch for seed {seed}");
        let checks=actual.iter().map(|(_,frame,final_state,digest)|format!("{{\"frame\":{frame},\"final\":{final_state},\"sha256\":{}}}",json(digest))).collect::<Vec<_>>().join(",");
        writeln!(output,"{{\"seed\":{seed},\"equal\":true,\"checkpoints\":[{checks}],\"baseline\":{},\"normal\":{}}}",baseline.json(),normal.json())?;
        output.flush()?;
        println!("seed={seed} equal; frames={} stop={:?}; {} complete-state checkpoints",baseline.reached,baseline.stop,actual.len());
    }
    Ok(())
}
