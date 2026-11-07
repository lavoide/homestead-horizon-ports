// The crafting grid find_with was last given, on this thread, for the Comforts-first lookup.
// Injected as `crate::crafting::comforts` (horizon_client_lib).
use super::*;

thread_local! {
    static GRID: std::cell::RefCell<(Vec<Option<ItemStack>>, usize)> =
        const { std::cell::RefCell::new((Vec::new(), 0)) };
}
pub(crate) fn stash(cells: &[Option<ItemStack>], width: usize) {
    GRID.with(|g| *g.borrow_mut() = (cells.to_vec(), width));
}
pub(crate) fn grid() -> (Vec<Option<ItemStack>>, usize) {
    GRID.with(|g| g.borrow().clone())
}
