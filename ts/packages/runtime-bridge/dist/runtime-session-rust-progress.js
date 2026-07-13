/**
 * Owns facade-local progress bookkeeping. Session time and projection time are
 * deliberately separate: authority can advance without publishing a G1 frame.
 */
export class RuntimeSessionProgress {
    #sequenceId = 0;
    #sessionTick = 0;
    #latestProjectionTick = 0;
    #acceptedCommandCount = 0;
    #rejectedCommandCount = 0;
    #restartCount = 0;
    initialize() {
        this.#sequenceId = 0;
        this.#sessionTick = 0;
        this.#latestProjectionTick = 0;
        this.#acceptedCommandCount = 0;
        this.#rejectedCommandCount = 0;
        this.#restartCount = 0;
    }
    get sequenceId() { return this.#sequenceId; }
    get sessionTick() { return this.#sessionTick; }
    get latestProjectionTick() { return this.#latestProjectionTick; }
    get acceptedCommandCount() { return this.#acceptedCommandCount; }
    get rejectedCommandCount() { return this.#rejectedCommandCount; }
    get restartCount() { return this.#restartCount; }
    snapshot() {
        return {
            sequenceId: this.#sequenceId,
            sessionTick: this.#sessionTick,
            latestProjectionTick: this.#latestProjectionTick,
            acceptedCommandCount: this.#acceptedCommandCount,
            rejectedCommandCount: this.#rejectedCommandCount,
            restartCount: this.#restartCount,
        };
    }
    nextSimulationTick(requested) {
        return requested ?? this.#sessionTick + 1;
    }
    advanceSequence() {
        this.#sequenceId += 1;
        return this.#sequenceId;
    }
    recordCommandBatch(accepted, rejected) {
        this.#acceptedCommandCount += accepted;
        this.#rejectedCommandCount += rejected;
        this.advanceSequence();
    }
    recordSimulationTick(tick) {
        this.#sessionTick = tick;
        this.advanceSequence();
    }
    observeAuthorityTick(tick) {
        this.#sessionTick = Math.max(this.#sessionTick, tick);
    }
    recordProjectionTick(tick) {
        this.#latestProjectionTick = tick;
    }
    recordProjectedAuthorityTick(tick) {
        this.recordProjectionTick(tick);
        this.observeAuthorityTick(tick);
    }
    restart() {
        this.advanceSequence();
        this.#sessionTick = 0;
        this.#latestProjectionTick = 0;
        this.#acceptedCommandCount = 0;
        this.#rejectedCommandCount = 0;
        this.#restartCount += 1;
    }
}
//# sourceMappingURL=runtime-session-rust-progress.js.map