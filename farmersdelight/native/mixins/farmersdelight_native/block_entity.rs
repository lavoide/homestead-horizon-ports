// EntityBlock.newBlockEntity for Farmer's Delight's machines.
fn fd_block_entity(block: &str, entity: &mut Option<(String, Compound)>) {
    crate::game::farmersdelight::block_entity(block, entity);
}
