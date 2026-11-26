use bevy::prelude::*;
use sim::Sim;
use std::path::{Path, PathBuf};

pub const RTS_TICKS_PER_SECOND: f64 = 16.0;

#[derive(Resource)]
pub struct RtsSim(pub Sim);

#[derive(Resource, Default)]
pub struct RtsHashes(pub Vec<(u32, u64)>);

pub struct RtsSimPlugin {
    pub image: PathBuf,
    pub max_units: usize,
}

impl Plugin for RtsSimPlugin {
    fn build(&self, app: &mut App) {
        let sim = sim::load_file(Path::new(&self.image), self.max_units)
            .unwrap_or_else(|error| panic!("rts sim image {}: {error:?}", self.image.display()));
        app.insert_resource(Time::<Fixed>::from_hz(RTS_TICKS_PER_SECOND))
            .insert_resource(RtsSim(sim))
            .init_resource::<RtsHashes>()
            .add_systems(FixedUpdate, step_rts_sim);
    }
}

fn step_rts_sim(mut rts: ResMut<RtsSim>, mut hashes: ResMut<RtsHashes>) {
    rts.0.tick();
    hashes.0.push((rts.0.ticks(), rts.0.state_hash()));
}
