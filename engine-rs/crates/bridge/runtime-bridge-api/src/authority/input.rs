use super::*;

pub(super) fn configure(
    bridge: &mut EngineBridge,
    request: InputSessionConfigureRequest,
) -> BridgeResult<InputSessionSnapshot> {
    bridge.require_initialized("configure_input_session")?;
    let resolver = InputSessionResolver::activate(request.catalog, request.initial_contexts)
        .map_err(|error| {
            let details = error
                .diagnostics()
                .iter()
                .map(|item| format!("{:?}@{}: {}", item.code, item.path, item.message))
                .collect::<Vec<_>>()
                .join("; ");
            RuntimeBridgeError::new(
                RuntimeBridgeErrorKind::InvalidInput,
                format!("input Session activation rejected: {details}"),
            )
        })?;
    let snapshot = resolver.snapshot();
    bridge.input.input_session = Some(resolver);
    Ok(snapshot)
}

pub(super) fn apply_context_command(
    bridge: &mut EngineBridge,
    command: InputContextCommand,
) -> BridgeResult<InputContextChangeReceipt> {
    let resolver = bridge.input.input_session.as_mut().ok_or_else(|| {
        RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::NotInitialized,
            "apply_input_context_command called before configure_input_session",
        )
    })?;
    Ok(resolver.apply_context_command(command))
}

pub(super) fn submit(
    bridge: &EngineBridge,
    sample: RawInputSample,
) -> BridgeResult<InputResolutionReceipt> {
    let resolver = bridge.input.input_session.as_ref().ok_or_else(|| {
        RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::NotInitialized,
            "submit_raw_input called before configure_input_session",
        )
    })?;
    Ok(resolver.resolve(sample))
}

pub(super) fn replay(
    bridge: &mut EngineBridge,
    record: RecordedInputAction,
) -> BridgeResult<InputActionReplayReceipt> {
    let resolver = bridge.input.input_session.as_mut().ok_or_else(|| {
        RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::NotInitialized,
            "replay_resolved_input_action called before configure_input_session",
        )
    })?;
    Ok(resolver.replay(record))
}

pub(super) fn read_context_state(bridge: &EngineBridge) -> BridgeResult<InputContextStackState> {
    let resolver = bridge.input.input_session.as_ref().ok_or_else(|| {
        RuntimeBridgeError::new(
            RuntimeBridgeErrorKind::NotInitialized,
            "read_input_context_state called before configure_input_session",
        )
    })?;
    Ok(resolver.context_state().clone())
}
