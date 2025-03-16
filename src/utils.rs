use rosu_pp::{
    model::control_point::{EffectPoint, TimingPoint},
    Beatmap,
};

pub fn effect_point_at(beatmap: &Beatmap, time: f64) -> Option<&EffectPoint> {
    beatmap
        .effect_points
        .binary_search_by(|probe| probe.time.total_cmp(&time))
        .map_or_else(|i| i.checked_sub(1), Some)
        .map(|i| &beatmap.effect_points[i])
}

pub fn timing_point_at(beatmap: &Beatmap, time: f64) -> Option<&TimingPoint> {
    let i = beatmap
        .timing_points
        .binary_search_by(|probe| probe.time.total_cmp(&time))
        .unwrap_or_else(|i| i.saturating_sub(1));

    beatmap.timing_points.get(i)
}
