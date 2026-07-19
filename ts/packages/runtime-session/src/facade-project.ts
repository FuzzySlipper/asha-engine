import type {
  RuntimeProjectCloseReceipt,
  RuntimeProjectLoadReceipt,
} from '@asha/contracts';

/** Host byte reader selected by ordinary product boot code. The manifest read
 * through this adapter remains the sole owner of source closure and meaning. */
export interface RuntimeSessionProjectSource {
  readonly kind:
    | 'development-directory'
    | 'packaged-directory'
    | 'packaged-archive'
    | 'memory';
  readonly identity: string;
  read(relativePath: string): Promise<Uint8Array>;
}

export interface RuntimeSessionProjectLoadInput {
  readonly source: RuntimeSessionProjectSource;
}

export type RuntimeSessionProjectLoadReceipt = RuntimeProjectLoadReceipt;
export type RuntimeSessionProjectCloseReceipt = RuntimeProjectCloseReceipt;
