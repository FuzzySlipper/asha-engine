import { EditorResolvedInputConsumer, } from '@asha/editor-tools';
import { BrowserInputHost, } from '@asha/runtime-bridge';
/**
 * Production editor input composition. The browser host owns DOM normalization,
 * Session owns action resolution/context consumption, editor-tools owns the
 * expression accumulator, and app alone applies drained camera/tool intent.
 */
export class AppEditorInputComposition {
    host;
    #consumer = new EditorResolvedInputConsumer();
    #editor;
    #camera;
    constructor(options) {
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
    drain() {
        const frame = this.#consumer.drain();
        this.#camera.apply(frame);
        if (frame.cancelPressed) {
            this.#editor.cancel();
            return { frame, committed: null, cancelled: true };
        }
        const committed = frame.primaryToolPressed ? this.#editor.commit() : null;
        return { frame, committed, cancelled: false };
    }
    reset() {
        this.#consumer.reset();
    }
}
//# sourceMappingURL=editor-input-composition.js.map