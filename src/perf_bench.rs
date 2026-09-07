//! Ad-hoc perf diagnostic for dungeon-mob AI tick cost at scale - NOT a correctness test, and
//! not meant to stay in the codebase long-term. Added to investigate/verify a reported "~1000
//! mobs causes severe lag, `--release` makes it much smoother" symptom.
//!
//! Run with (from `Smalldungeons/`):
//!   cargo test --release perf_bench -- --nocapture     (release numbers)
//!   cargo test perf_bench -- --nocapture                (debug numbers)
//!
//! Measures pure server-side simulation cost: `World::tick()` wall time with 0/100/1000 idle
//! Skeleton Soldiers and nobody around to fight, isolated from network I/O and client rendering
//! entirely (no real client ever connects here). `world.server` is deliberately left null -
//! idle mobs (no player in range -> no target -> no combat/pathfinding/green-room code, all of
//! which are the only things that ever call `world.server_mut()` from the mob AI path) never
//! dereference it, so this is safe as long as every spawned mob here stays idle the whole time
//! (verified against the current `ai/mod.rs`/`spawner.rs` - re-check before reusing this if
//! those change to add another `server_mut()` call reachable from an idle mob).
#[cfg(test)]
mod tests {
    use crate::server::entity::dungeon_mobs::mob_type::{DungeonMobType, MobBaseKind};
    use crate::server::entity::dungeon_mobs::spawner::spawn_active_mob;
    use crate::server::entity::equipment::Equipment;
    use crate::server::utils::dvec3::DVec3;
    use crate::server::world::World;
    use std::time::Instant;

    fn bench(mob_count: usize, ticks: usize) {
        let mut world = World::new();

        let spacing = 1.5;
        let per_row = 32;
        let offset = (per_row as f64 - 1.0) * spacing / 2.0;
        for i in 0..mob_count {
            let row = (i / per_row) as f64;
            let col = (i % per_row) as f64;
            let pos = DVec3::new(col * spacing - offset, 70.0, row * spacing - offset);
            spawn_active_mob(
                &mut world,
                0,
                false,
                pos,
                0.0,
                Some(DungeonMobType::SkeletonSoldier),
                MobBaseKind::Skeleton,
                false,
                false,
                Equipment::default(),
                "bench".to_string(),
                "Skeleton Soldier".to_string(),
                None,
            );
        }

        // Warm up (first couple ticks can include one-off setup costs, e.g. the first
        // chunk-migration pass) before timing.
        for _ in 0..5 {
            world.tick().unwrap();
        }

        let start = Instant::now();
        for _ in 0..ticks {
            world.tick().unwrap();
        }
        let elapsed = start.elapsed();

        println!(
            "mobs={mob_count:5}  ticks={ticks:4}  total={:9.2}ms  avg_tick={:8.4}ms",
            elapsed.as_secs_f64() * 1000.0,
            elapsed.as_secs_f64() * 1000.0 / ticks as f64,
        );
    }

    #[test]
    fn perf_bench_mob_tick_time() {
        for &count in &[0usize, 100, 1000] {
            bench(count, 100);
        }
    }
}
