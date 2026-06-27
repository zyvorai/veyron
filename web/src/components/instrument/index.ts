/* ============================================================
   Instrument Deck component library — barrel export.
   Import individual components to keep bundles lean.
   ============================================================ */
export { SignalDot }         from './SignalDot';
export { MetricTile }        from './MetricTile';
export { Gauge }             from './Gauge';
export { VmCard }            from './VmCard';
export { AlertRow }          from './AlertRow';
export { TelemetrySpine }    from './TelemetrySpine';
export { ZeusPanel }         from './ZeusPanel';
export { SectionHeading }    from './SectionHeading';
export { Chip }              from './Chip';
export { ReactorCore }       from './ReactorCore';
export { TopologySurface }   from './TopologySurface';

export { useFleetTelemetry } from './useFleetTelemetry';

export type {
  Signal,
  FleetVM,
  VmApiRow,
  NodeGroup,
} from './types';

export {
  deriveSignal,
  adaptRow,
  fleetHealth,
  groupByNode,
} from './types';
