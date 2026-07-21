export interface RuntimeSessionProgressSnapshot {
  readonly sequenceId: number;
  readonly sessionTick: number;
  readonly latestProjectionTick: number;
  readonly acceptedCommandCount: number;
  readonly rejectedCommandCount: number;
  readonly restartCount: number;
}

/**
 * Owns facade-local progress bookkeeping. Session time and projection time are
 * deliberately separate: authority can advance without publishing a G1 frame.
 */
export class RuntimeSessionProgress {
  #sequenceId = 0;
  #sessionTick = 0;
  #latestProjectionTick = 0;
  #nextProjectionCursor = 0;
  #acceptedCommandCount = 0;
  #rejectedCommandCount = 0;
  #restartCount = 0;

  initialize(): void {
    this.#sequenceId = 0;
    this.#sessionTick = 0;
    this.#latestProjectionTick = 0;
    this.#nextProjectionCursor = 0;
    this.#acceptedCommandCount = 0;
    this.#rejectedCommandCount = 0;
    this.#restartCount = 0;
  }

  get sequenceId(): number { return this.#sequenceId; }
  get sessionTick(): number { return this.#sessionTick; }
  get latestProjectionTick(): number { return this.#latestProjectionTick; }
  get acceptedCommandCount(): number { return this.#acceptedCommandCount; }
  get rejectedCommandCount(): number { return this.#rejectedCommandCount; }
  get restartCount(): number { return this.#restartCount; }

  snapshot(): RuntimeSessionProgressSnapshot {
    return {
      sequenceId: this.#sequenceId,
      sessionTick: this.#sessionTick,
      latestProjectionTick: this.#latestProjectionTick,
      acceptedCommandCount: this.#acceptedCommandCount,
      rejectedCommandCount: this.#rejectedCommandCount,
      restartCount: this.#restartCount,
    };
  }

  nextSimulationTick(requested?: number): number {
    return requested ?? this.#sessionTick + 1;
  }

  advanceSequence(): number {
    this.#sequenceId += 1;
    return this.#sequenceId;
  }

  recordCommandBatch(accepted: number, rejected: number): void {
    this.#acceptedCommandCount += accepted;
    this.#rejectedCommandCount += rejected;
    this.advanceSequence();
  }

  recordSimulationTick(tick: number): void {
    this.#sessionTick = tick;
    this.advanceSequence();
  }

  observeAuthorityTick(tick: number): void {
    this.#sessionTick = Math.max(this.#sessionTick, tick);
  }

  recordProjectionTick(tick: number): void {
    this.#latestProjectionTick = tick;
  }

  recordProjectedAuthorityTick(tick: number): void {
    this.recordProjectionTick(tick);
    this.observeAuthorityTick(tick);
  }

  claimProjectionCursor(): number {
    if (this.#nextProjectionCursor >= Number.MAX_SAFE_INTEGER) {
      throw new RangeError('runtime projection cursor exhausted the safe integer range');
    }
    const cursor = this.#nextProjectionCursor;
    this.#nextProjectionCursor += 1;
    return cursor;
  }

  restart(): void {
    this.advanceSequence();
    this.#sessionTick = 0;
    this.#latestProjectionTick = 0;
    this.#acceptedCommandCount = 0;
    this.#rejectedCommandCount = 0;
    this.#restartCount += 1;
  }
}
