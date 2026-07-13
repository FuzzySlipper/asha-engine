// Browser/canvas surface built on the retained ASHA ThreeRenderer.
import * as THREE from 'three';
import { RenderProjection, createGeneratedTunnelViewportFrame, summarizeFirstPersonTunnelViewport, } from '@asha/render-projection';
import { renderHandle } from '@asha/contracts';
import { ThreeRenderer } from './three-renderer.js';
/**
 * Apply a render frame through the renderer-neutral projection and then the
 * retained Three.js renderer. This is the package-root bridge used by demo
 * proofs: no authority state, no raw transport, no arbitrary JSON tunnel.
 */
export function renderProjectedFrame(frame, renderer = new ThreeRenderer()) {
    const projection = new RenderProjection();
    projection.applyFrame(frame);
    renderer.applyFrame(frame);
    return {
        projection,
        renderer,
        structuralSnapshot: renderer.snapshot(),
    };
}
export function renderFirstPersonTunnelViewport(input, renderer = new ThreeRenderer()) {
    const frame = createGeneratedTunnelViewportFrame(input.tunnel, input.materials);
    const rendered = renderProjectedFrame(frame, renderer);
    return {
        ...rendered,
        frame,
        summary: summarizeFirstPersonTunnelViewport({
            tunnel: input.tunnel,
            camera: input.camera,
            frame,
            structuralSnapshot: rendered.structuralSnapshot,
            ...(input.collision === undefined ? {} : { collision: input.collision }),
        }),
    };
}
/**
 * A tiny public browser surface for consumers that need to prove the real
 * renderer path: ASHA render diffs -> retained ThreeRenderer -> WebGL canvas.
 *
 * The consumer owns only the canvas element. Three.js scene/camera/WebGL details
 * stay inside `@asha/renderer-three`.
 */
export function mountAshaRendererBrowserSurface(canvas, options = {}) {
    const renderer = new ThreeRenderer(options.animatedMeshSource === undefined ? {} : { animatedMeshSource: options.animatedMeshSource });
    // Catalog materials use MeshStandardMaterial. Keep the browser host responsible
    // for a small neutral light rig; the retained projection carries appearance
    // parameters, never renderer-owned light state or gameplay authority.
    const ambientLight = new THREE.HemisphereLight(0xffffff, 0x263238, 2.4);
    const keyLight = new THREE.DirectionalLight(0xffffff, 2.2);
    keyLight.position.set(5, 8, 6);
    renderer.scene.add(ambientLight, keyLight);
    const frame = options.frame ?? createAshaRendererBrowserSurfaceFrame();
    renderer.applyFrame(frame);
    const webgl = new THREE.WebGLRenderer({ canvas, antialias: true });
    webgl.setClearColor(options.clearColor ?? 0x101820, 1);
    webgl.setPixelRatio(options.pixelRatio ?? globalThis.devicePixelRatio ?? 1);
    const camera = new THREE.PerspectiveCamera(55, 1, 0.1, 100);
    const raycaster = new THREE.Raycaster();
    const center = new THREE.Vector2(0, 0);
    const cameraLookTarget = new THREE.Vector3();
    let currentCameraPose = options.camera?.initialPose ?? {
        position: [0, 1.62, 8],
        pitchDegrees: 0,
        yawDegrees: 0,
    };
    let currentCameraBasis = options.camera?.initialBasis ?? null;
    let animationFrame = null;
    let lastRenderTimeMs = null;
    const setCameraPose = (pose, basis) => {
        currentCameraPose = pose;
        currentCameraBasis = basis ?? null;
        camera.position.set(pose.position[0], pose.position[1], pose.position[2]);
        if (currentCameraBasis === null) {
            camera.up.set(0, 1, 0);
            camera.rotation.order = 'YXZ';
            camera.rotation.x = degreesToRadians(pose.pitchDegrees);
            camera.rotation.y = degreesToRadians(pose.yawDegrees);
            camera.rotation.z = 0;
            return;
        }
        camera.up.set(currentCameraBasis.up[0], currentCameraBasis.up[1], currentCameraBasis.up[2]);
        cameraLookTarget.set(camera.position.x + currentCameraBasis.forward[0], camera.position.y + currentCameraBasis.forward[1], camera.position.z + currentCameraBasis.forward[2]);
        camera.lookAt(cameraLookTarget);
    };
    const resize = () => {
        const width = Math.max(1, canvas.clientWidth || canvas.width || 800);
        const height = Math.max(1, canvas.clientHeight || canvas.height || 450);
        if (canvas.width !== width || canvas.height !== height) {
            webgl.setSize(width, height, false);
        }
        camera.aspect = width / height;
        camera.updateProjectionMatrix();
    };
    const renderOnce = (timeMs = globalThis.performance?.now() ?? 0) => {
        resize();
        const deltaSeconds = lastRenderTimeMs === null
            ? 0
            : Math.min(0.05, Math.max(0, (timeMs - lastRenderTimeMs) / 1000));
        lastRenderTimeMs = timeMs;
        renderer.advanceAnimation(deltaSeconds);
        webgl.render(renderer.scene, camera);
    };
    const tick = (timeMs) => {
        renderOnce(timeMs);
        animationFrame = globalThis.requestAnimationFrame(tick);
    };
    const start = () => {
        if (animationFrame !== null) {
            return;
        }
        animationFrame = globalThis.requestAnimationFrame(tick);
    };
    const stop = () => {
        if (animationFrame === null) {
            return;
        }
        globalThis.cancelAnimationFrame(animationFrame);
        animationFrame = null;
    };
    const dispose = () => {
        stop();
        webgl.dispose();
    };
    setCameraPose(currentCameraPose, currentCameraBasis ?? undefined);
    renderOnce(0);
    if (options.autoStart !== false) {
        start();
    }
    return {
        kind: 'asha_renderer_browser_surface.v0',
        canvas,
        renderer,
        frame,
        animatedMeshPlayback: (handle) => renderer.animatedMeshPlayback(handle),
        applyFrame: (nextFrame) => renderer.applyFrame(nextFrame),
        cameraPose: () => currentCameraPose,
        pickCenterObject: (request) => pickCenterObject(renderer.scene, camera, raycaster, center, request),
        projectObjectProjection: (projection) => projectObjectProjection(renderer.scene, projection),
        snapshot: () => renderer.snapshot(),
        renderOnce,
        setCameraPose,
        start,
        stop,
        dispose,
    };
}
export function createAshaRendererBrowserSurfaceFrame() {
    const cubeSpecs = createBrowserSurfaceCubeSpecs();
    return {
        ops: [
            {
                op: 'create',
                handle: renderHandle(4103001),
                parent: null,
                node: primitiveNode('asha-renderer-flat-plane', 'cube', [0, -0.08, 0], [18, 0.16, 18], [
                    0.16,
                    0.22,
                    0.2,
                    1,
                ]),
            },
            {
                op: 'create',
                handle: renderHandle(4103002),
                parent: null,
                node: primitiveNode('asha-renderer-collision-wall-north', 'cube', [0, 0.5, -2.5], [6, 3, 1], [
                    0.32,
                    0.38,
                    0.42,
                    1,
                ]),
            },
            {
                op: 'create',
                handle: renderHandle(4103003),
                parent: null,
                node: primitiveNode('asha-renderer-collision-wall-south', 'cube', [0, 0.5, 2.5], [6, 3, 1], [
                    0.32,
                    0.38,
                    0.42,
                    1,
                ]),
            },
            {
                op: 'create',
                handle: renderHandle(4103004),
                parent: null,
                node: primitiveNode('asha-renderer-collision-wall-west', 'cube', [-2.5, 0.5, 0], [1, 3, 6], [
                    0.27,
                    0.34,
                    0.37,
                    1,
                ]),
            },
            {
                op: 'create',
                handle: renderHandle(4103005),
                parent: null,
                node: primitiveNode('asha-renderer-collision-wall-east', 'cube', [2.5, 0.5, 0], [1, 3, 6], [
                    0.27,
                    0.34,
                    0.37,
                    1,
                ]),
            },
            ...cubeSpecs.map((cube, index) => ({
                op: 'create',
                handle: renderHandle(4103100 + index),
                parent: null,
                node: primitiveNode(`asha-renderer-random-cube-${String(index + 1).padStart(2, '0')}`, 'cube', [cube.position[0], cube.size[1] / 2, cube.position[1]], cube.size, cube.color),
            })),
        ],
    };
}
function pickCenterObject(scene, camera, raycaster, center, request) {
    const requestedLabels = new Set(request.labels);
    const meshes = collectLabeledMeshes(scene, requestedLabels);
    if (meshes.length === 0) {
        return null;
    }
    scene.updateMatrixWorld(true);
    raycaster.setFromCamera(center, camera);
    const intersection = raycaster.intersectObjects(meshes.map((target) => target.mesh), false)[0];
    if (intersection === undefined) {
        return null;
    }
    const picked = meshes.find((candidate) => candidate.mesh === intersection.object);
    if (picked === undefined) {
        return null;
    }
    return {
        distance: Number(intersection.distance.toFixed(2)),
        label: picked.label,
    };
}
function projectObjectProjection(scene, projection) {
    const [target] = collectLabeledMeshes(scene, new Set([projection.label]));
    if (target === undefined) {
        return;
    }
    target.mesh.visible = projection.visible;
    if (projection.position !== undefined) {
        target.mesh.position.set(projection.position[0], projection.position[1], projection.position[2]);
    }
    if (projection.scale !== undefined) {
        target.mesh.scale.set(projection.scale[0], projection.scale[1], projection.scale[2]);
    }
    if (projection.color !== undefined) {
        target.material.color.setRGB(projection.color[0], projection.color[1], projection.color[2]);
        return;
    }
    if (projection.visible) {
        target.material.color.copy(target.baseColor);
    }
}
function collectLabeledMeshes(scene, labels) {
    const targets = [];
    scene.traverse((object) => {
        if (!labels.has(object.name)) {
            return;
        }
        const mesh = object;
        const material = Array.isArray(mesh.material) ? mesh.material[0] : mesh.material;
        if (!(material instanceof THREE.MeshBasicMaterial) && !(material instanceof THREE.MeshStandardMaterial)) {
            return;
        }
        targets.push({
            baseColor: material.color.clone(),
            label: object.name,
            material,
            mesh,
        });
    });
    return targets;
}
function createBrowserSurfaceCubeSpecs() {
    const random = deterministicUnitGenerator(0x4103c0de);
    const colors = [
        [0.28, 0.66, 0.92, 1],
        [0.92, 0.54, 0.32, 1],
        [0.46, 0.78, 0.42, 1],
        [0.82, 0.58, 0.92, 1],
        [0.92, 0.76, 0.28, 1],
    ];
    const cubes = [
        {
            color: colors[0],
            position: [0, -1.35],
            size: [0.62, 2.2, 0.62],
        },
        {
            color: colors[1],
            position: [1.25, -0.65],
            size: [0.48, 0.85, 0.48],
        },
        {
            color: colors[2],
            position: [-1.15, -0.9],
            size: [0.52, 1.05, 0.52],
        },
        {
            color: colors[3],
            position: [0.85, 1.1],
            size: [0.44, 0.75, 0.44],
        },
    ];
    for (let index = cubes.length; index < 28; index += 1) {
        const width = round2(0.55 + random() * 1.55);
        const height = round2(0.65 + random() * 2.8);
        const depth = round2(0.55 + random() * 1.55);
        let x = round2(-7 + random() * 14);
        let z = round2(-7 + random() * 14);
        if (x > -3.5 && x < 3.5 && z > -3.5 && z < 3.5) {
            z = round2(z < 0 ? z - 3.75 : z + 3.75);
        }
        cubes.push({
            color: colors[index % colors.length],
            position: [x, z],
            size: [width, height, depth],
        });
    }
    return cubes;
}
function primitiveNode(label, shape, translation, scale, color) {
    return {
        geometry: { shape },
        material: { color, wireframe: false },
        transform: identityTransform(translation, scale),
        visible: true,
        layer: 'scene',
        metadata: { source: null, tags: [], label },
    };
}
function identityTransform(translation, scale) {
    return {
        translation,
        rotation: [0, 0, 0, 1],
        scale,
    };
}
function deterministicUnitGenerator(seed) {
    let state = seed >>> 0;
    return () => {
        state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
        return state / 0x100000000;
    };
}
function degreesToRadians(degrees) {
    return (degrees * Math.PI) / 180;
}
function round2(value) {
    return Number(value.toFixed(2));
}
// ── Snapshot lines (deterministic golden artifact) ────────────────────────────
//# sourceMappingURL=browser-surface.js.map