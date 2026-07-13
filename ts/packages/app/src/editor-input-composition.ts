import type { VoxelCommand } from '@asha/contracts';
import {
  EditorResolvedInputConsumer,
  type EditorResolvedInputFrame,
} from '@asha/editor-tools';
import {
  BrowserInputHost,
  type BrowserInputSessionPort,
} from '@asha/runtime-bridge';

export interface EditorCameraInputPort {
  apply(frame: EditorResolvedInputFrame): void;
}

export interface EditorToolInputPort {
  commit(): VoxelCommand | null;
  cancel(): void;
}

export interface AppEditorInputCompositionOptions {
  readonly session: BrowserInputSessionPort;
  readonly editor: EditorToolInputPort;
  readonly camera: EditorCameraInputPort;
}

export interface AppEditorInputDrainReceipt {
  readonly frame: EditorResolvedInputFrame;
  readonly committed: VoxelCommand | null;
  readonly cancelled: boolean;
}

/**
 * Production editor input composition. The browser host owns DOM normalization,
 * Session owns action resolution/context consumption, editor-tools owns the
 * expression accumulator, and app alone applies drained camera/tool intent.
 */
export class AppEditorInputComposition {
  readonly host: BrowserInputHost;
  readonly #consumer = new EditorResolvedInputConsumer();
  readonly #editor: EditorToolInputPort;
  readonly #camera: EditorCameraInputPort;

  constructor(options: AppEditorInputCompositionOptions) {
    this.#editor = options.editor;
    this.#camera = options.camera;
    this.host = new BrowserInputHost({
      session: options.session,
      initialContexts: ['editor'],
      consumers: {
        'editor.camera.forward': 'app.editorCamera',
        'editor.camera.backward': 'app.editorCamera',
        'editor.camera.left': 'app.editorCamera',
        'editor.camera.right': 'app.editorCamera',
        'editor.camera.look': 'app.editorCamera',
        'editor.tool.primary': 'app.editorTools',
        'editor.tool.cancel': 'app.editorTools',
      },
      onResolvedAction: (action) => this.#consumer.accept(action),
      onContextChanged: () => this.#consumer.reset(),
    });
  }

  drain(): AppEditorInputDrainReceipt {
    const frame = this.#consumer.drain();
    this.#camera.apply(frame);
    if (frame.cancelPressed) {
      this.#editor.cancel();
      return { frame, committed: null, cancelled: true };
    }
    const committed = frame.primaryToolPressed ? this.#editor.commit() : null;
    return { frame, committed, cancelled: false };
  }

  reset(): void {
    this.#consumer.reset();
  }
}
