// A chunk came: a new overworld one is queued for a bridge.
fn yungsbridges_chunk(dimension: &str, chunk: [i32; 2], new: bool) {
    crate::game::yungsbridges::chunk_loaded(dimension, chunk, new);
}
