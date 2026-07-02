export type TelemetrySource = 'runtime' | 'policy' | 'renderer' | 'devtools' | 'replay';
export type TelemetryLevel = 'debug' | 'info' | 'warning' | 'error';
export type TelemetryMetricKind = 'counter' | 'gauge' | 'durationMs';
export interface TelemetryMetric {
    readonly name: string;
    readonly kind: TelemetryMetricKind;
    readonly value: number;
    readonly unit: string | null;
}
export type TelemetryEvent = {
    readonly kind: 'metric';
    readonly source: TelemetrySource;
    readonly level: TelemetryLevel;
    readonly sequence: number;
    readonly metric: TelemetryMetric;
} | {
    readonly kind: 'trace';
    readonly source: TelemetrySource;
    readonly level: TelemetryLevel;
    readonly sequence: number;
    readonly span: string;
    readonly message: string;
};
export interface TelemetryEnvelope {
    readonly protocolVersion: number;
    readonly emittedAtTick: number;
    readonly events: readonly TelemetryEvent[];
}
//# sourceMappingURL=telemetry.d.ts.map