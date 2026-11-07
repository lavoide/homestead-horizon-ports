fn secure(root: &Path, properties: &MovementProperties<'_>) -> core::ops::ControlFlow<Result<BlockLandingRegistry>> {
    core::ops::ControlFlow::Break(crate::landing::securecrops::load(root, properties))
}
