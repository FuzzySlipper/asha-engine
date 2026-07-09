#!/usr/bin/env node

const childProcess = require('node:child_process');
const crypto = require('node:crypto');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const FBX2GLTF_VERSION = '0.9.7-p1';
const repoRoot = path.resolve(__dirname, '../../..');
const sourceRoot = process.env.ASHA_MESH_ANIMATION_SOURCE_ROOT || '/home/stash/mesh-resources';
const fixtureDir = path.join(repoRoot, 'harness/fixtures/mesh-animation');
const packageDir =
  process.env.ASHA_FBX2GLTF_PACKAGE_DIR ||
  path.join(os.tmpdir(), `asha-fbx2gltf-${FBX2GLTF_VERSION}`);

const sourceFiles = {
  model: {
    path: 'Model/characterMedium.fbx',
    sha256: '18835fef534eede635b081ee7fe647d01a885550a591d2e6bf071010906167d8',
  },
  idle: {
    path: 'Animations/idle.fbx',
    sha256: 'c8a24e0294376ee5a195c56752a13310e1c0b5f8588a4db50e094120e3e4cc74',
    sourceAnimationName: 'Root|Idle',
  },
  run: {
    path: 'Animations/run.fbx',
    sha256: 'e635461fc8dace85ec67a7f7941e949a7c3f108b51ae4d2da1557e6e01749df8',
    sourceAnimationName: 'Root|Run',
  },
  jump: {
    path: 'Animations/jump.fbx',
    sha256: 'b88429077a7a1af5d3f55f43cfd8ce0f7441b4f6f7bb15a8070d7ed15d275f74',
    sourceAnimationName: 'Root|Jump',
  },
  texture: {
    path: 'Skins/humanMaleA.png',
    sha256: '1590e08cea37f5aecbacabb40a57c176e389e9a95d5b2a4de00086604ef23e1c',
  },
  license: {
    path: 'License.txt',
    sha256: 'eaa916e20df30c26f18a752290f93ab0e5d95c3dd1057e6887d11aa4acc0e74b',
  },
};

function fail(message) {
  console.error(`FAIL: ${message}`);
  process.exit(1);
}

function align4(n) {
  return (n + 3) & ~3;
}

function clone(value) {
  return JSON.parse(JSON.stringify(value));
}

function sha256File(filePath) {
  return crypto.createHash('sha256').update(fs.readFileSync(filePath)).digest('hex');
}

function verifySources() {
  for (const [id, source] of Object.entries(sourceFiles)) {
    const filePath = path.join(sourceRoot, source.path);
    if (!fs.existsSync(filePath)) {
      fail(`missing ${id} source at ${filePath}`);
    }
    const actual = sha256File(filePath);
    if (actual !== source.sha256) {
      fail(`${id} source hash drifted: expected ${source.sha256}, got ${actual}`);
    }
  }
}

function ensureConverter() {
  const packageJson = path.join(packageDir, 'node_modules/fbx2gltf/package.json');
  if (fs.existsSync(packageJson)) {
    const installed = JSON.parse(fs.readFileSync(packageJson, 'utf8')).version;
    if (installed === FBX2GLTF_VERSION) {
      return require(path.join(packageDir, 'node_modules/fbx2gltf'));
    }
  }
  fs.mkdirSync(packageDir, { recursive: true });
  if (!fs.existsSync(path.join(packageDir, 'package.json'))) {
    childProcess.execFileSync('npm', ['init', '-y'], {
      cwd: packageDir,
      stdio: ['ignore', 'ignore', 'inherit'],
    });
  }
  childProcess.execFileSync('npm', ['install', `fbx2gltf@${FBX2GLTF_VERSION}`], {
    cwd: packageDir,
    stdio: ['ignore', 'inherit', 'inherit'],
  });
  return require(path.join(packageDir, 'node_modules/fbx2gltf'));
}

function readGlb(filePath) {
  const bytes = fs.readFileSync(filePath);
  if (bytes.toString('utf8', 0, 4) !== 'glTF') {
    fail(`${filePath} is not a GLB`);
  }
  const jsonLength = bytes.readUInt32LE(12);
  const jsonType = bytes.toString('utf8', 16, 20);
  if (jsonType !== 'JSON') {
    fail(`${filePath} first chunk is ${jsonType}, expected JSON`);
  }
  const json = JSON.parse(bytes.toString('utf8', 20, 20 + jsonLength).trim());
  let bin = Buffer.alloc(0);
  const binHeaderOffset = 20 + jsonLength;
  if (binHeaderOffset + 8 <= bytes.length) {
    const binLength = bytes.readUInt32LE(binHeaderOffset);
    const binType = bytes.toString('utf8', binHeaderOffset + 4, binHeaderOffset + 8);
    if (binType === 'BIN\0') {
      bin = bytes.subarray(binHeaderOffset + 8, binHeaderOffset + 8 + binLength);
    }
  }
  return { json, bin };
}

function writeGlb(filePath, json, bin) {
  json.buffers = [{ byteLength: bin.length }];
  const jsonRaw = Buffer.from(JSON.stringify(json, null, 2), 'utf8');
  const jsonChunk = Buffer.concat([jsonRaw, Buffer.alloc(align4(jsonRaw.length) - jsonRaw.length, 0x20)]);
  const binChunk = Buffer.concat([bin, Buffer.alloc(align4(bin.length) - bin.length)]);
  const totalLength = 12 + 8 + jsonChunk.length + 8 + binChunk.length;

  const header = Buffer.alloc(12);
  header.write('glTF', 0, 4, 'utf8');
  header.writeUInt32LE(2, 4);
  header.writeUInt32LE(totalLength, 8);

  const jsonHeader = Buffer.alloc(8);
  jsonHeader.writeUInt32LE(jsonChunk.length, 0);
  jsonHeader.write('JSON', 4, 4, 'utf8');

  const binHeader = Buffer.alloc(8);
  binHeader.writeUInt32LE(binChunk.length, 0);
  binHeader.write('BIN\0', 4, 4, 'utf8');

  fs.writeFileSync(filePath, Buffer.concat([header, jsonHeader, jsonChunk, binHeader, binChunk]));
}

function appendBinary(current, nextBytes) {
  const offset = align4(current.length);
  const padding = offset - current.length;
  const padded = padding > 0 ? Buffer.concat([current, Buffer.alloc(padding)]) : current;
  return { offset, bytes: Buffer.concat([padded, nextBytes]) };
}

function mergeAnimationClip(base, currentBin, clipId, animationGlb) {
  const source = readGlb(animationGlb);
  const sourceSpec = sourceFiles[clipId];
  const animation = source.json.animations?.find((candidate) => candidate.name === sourceSpec.sourceAnimationName);
  if (!animation) {
    fail(`${animationGlb} does not contain ${sourceSpec.sourceAnimationName}`);
  }

  const baseNodeByName = new Map((base.nodes || []).map((node, index) => [node.name, index]));
  const bufferViewOffset = base.bufferViews.length;
  const accessorOffset = base.accessors.length;
  const appended = appendBinary(currentBin, source.bin);

  base.bufferViews.push(
    ...(source.json.bufferViews || []).map((view) => ({
      ...clone(view),
      buffer: 0,
      byteOffset: (view.byteOffset || 0) + appended.offset,
    })),
  );
  base.accessors.push(
    ...(source.json.accessors || []).map((accessor) => {
      const next = clone(accessor);
      if (typeof next.bufferView === 'number') {
        next.bufferView += bufferViewOffset;
      }
      return next;
    }),
  );

  const copied = clone(animation);
  copied.name = clipId;
  for (const sampler of copied.samplers || []) {
    sampler.input += accessorOffset;
    sampler.output += accessorOffset;
  }
  for (const channel of copied.channels || []) {
    const sourceNode = source.json.nodes?.[channel.target.node];
    const mapped = sourceNode?.name ? baseNodeByName.get(sourceNode.name) : undefined;
    if (mapped === undefined) {
      fail(`${clipId} channel target ${channel.target.node} (${sourceNode?.name || 'unnamed'}) does not exist in model rig`);
    }
    channel.target.node = mapped;
  }
  base.animations.push(copied);
  return appended.bytes;
}

function animationDurationSeconds(gltf, animation) {
  let duration = 0;
  for (const sampler of animation.samplers || []) {
    const input = gltf.accessors?.[sampler.input];
    if (input?.max?.[0] > duration) {
      duration = input.max[0];
    }
  }
  return duration;
}

function vec3Bounds(gltf) {
  const min = [Infinity, Infinity, Infinity];
  const max = [-Infinity, -Infinity, -Infinity];
  for (const accessor of gltf.accessors || []) {
    if (accessor.type === 'VEC3' && Array.isArray(accessor.min) && Array.isArray(accessor.max)) {
      for (let i = 0; i < 3; i += 1) {
        min[i] = Math.min(min[i], accessor.min[i]);
        max[i] = Math.max(max[i], accessor.max[i]);
      }
    }
  }
  return { min, max };
}

async function main() {
  verifySources();
  const convert = ensureConverter();
  const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'asha-mesh-animation-'));
  const converted = {
    model: path.join(scratch, 'characterMedium.glb'),
    idle: path.join(scratch, 'idle.glb'),
    run: path.join(scratch, 'run.glb'),
    jump: path.join(scratch, 'jump.glb'),
  };

  await convert(path.join(sourceRoot, sourceFiles.model.path), converted.model, ['--khr-materials-unlit']);
  for (const clipId of ['idle', 'run', 'jump']) {
    await convert(path.join(sourceRoot, sourceFiles[clipId].path), converted[clipId], ['--khr-materials-unlit']);
  }

  const model = readGlb(converted.model);
  const gltf = model.json;
  gltf.asset = {
    ...gltf.asset,
    generator: `ASHA mesh-animation fixture prep using FBX2glTF ${FBX2GLTF_VERSION}`,
  };
  gltf.animations = [];
  gltf.bufferViews ||= [];
  gltf.accessors ||= [];

  let bin = Buffer.from(model.bin);
  for (const clipId of ['idle', 'run', 'jump']) {
    bin = mergeAnimationClip(gltf, bin, clipId, converted[clipId]);
  }

  const textureBytes = fs.readFileSync(path.join(sourceRoot, sourceFiles.texture.path));
  const textureAppend = appendBinary(bin, textureBytes);
  bin = textureAppend.bytes;
  const imageBufferView = gltf.bufferViews.length;
  gltf.bufferViews.push({
    buffer: 0,
    byteOffset: textureAppend.offset,
    byteLength: textureBytes.length,
  });
  gltf.images = [{ name: 'humanMaleA', mimeType: 'image/png', bufferView: imageBufferView }];
  gltf.samplers = [{ magFilter: 9728, minFilter: 9728, wrapS: 10497, wrapT: 10497 }];
  gltf.textures = [{ sampler: 0, source: 0 }];
  gltf.materials ||= [{ name: 'skin' }];
  gltf.materials[0].pbrMetallicRoughness ||= {};
  gltf.materials[0].pbrMetallicRoughness.baseColorTexture = { index: 0 };
  gltf.materials[0].pbrMetallicRoughness.baseColorFactor = [1, 1, 1, 1];

  fs.mkdirSync(fixtureDir, { recursive: true });
  const glbPath = path.join(fixtureDir, 'kenney-retro-character-medium.glb');
  writeGlb(glbPath, gltf, bin);
  const glbBytes = fs.readFileSync(glbPath);
  const contentHash = crypto.createHash('sha256').update(glbBytes).digest('hex');
  const manifest = {
    assetId: 'mesh-animation/kenney-retro-character-medium',
    sourcePackage: 'Kenney Animated Characters Retro 1.1',
    sourceLicense: 'CC0',
    sourceAttribution: 'Created/distributed by Kenney (www.kenney.nl)',
    runtimeFormat: 'glb',
    fixturePath: 'harness/fixtures/mesh-animation/kenney-retro-character-medium.glb',
    contentHashSha256: contentHash,
    sourceRoot,
    sourceHashes: Object.fromEntries(
      Object.entries(sourceFiles).map(([id, source]) => [id, { path: source.path, sha256: source.sha256 }]),
    ),
    clipIds: ['idle', 'run', 'jump'],
    primaryProofClipId: 'run',
    scaleHint: 1.0,
    boundsHint: vec3Bounds(gltf),
    clips: gltf.animations.map((animation) => ({
      id: animation.name,
      durationSeconds: animationDurationSeconds(gltf, animation),
      channelCount: animation.channels?.length || 0,
      samplerCount: animation.samplers?.length || 0,
    })),
    meshCount: gltf.meshes?.length || 0,
    skinCount: gltf.skins?.length || 0,
    nodeCount: gltf.nodes?.length || 0,
    imageCount: gltf.images?.length || 0,
    materialCount: gltf.materials?.length || 0,
    externalUris: ['buffers', 'images'].flatMap((key) => (gltf[key] || []).map((entry) => entry.uri).filter(Boolean)),
    conversion: {
      converterPackage: 'fbx2gltf',
      converterVersion: FBX2GLTF_VERSION,
      notes: [
        'FBX source files are source/import material only; runtime consumers load the committed GLB fixture.',
        'Animation targets are remapped from the separate animation FBX rigs to the model rig by node name.',
        'Clip ids are normalized to lowercase ASHA ids: idle, run, jump.',
      ],
    },
  };
  fs.writeFileSync(
    path.join(fixtureDir, 'kenney-retro-character-medium.manifest.json'),
    `${JSON.stringify(manifest, null, 2)}\n`,
  );
  const licenseText = fs
    .readFileSync(path.join(sourceRoot, sourceFiles.license.path), 'utf8')
    .replace(/\r\n?/g, '\n')
    .split('\n')
    .map((line) => line.replace(/[ \t]+$/u, ''))
    .join('\n')
    .replace(/\n*$/u, '\n');
  fs.writeFileSync(
    path.join(fixtureDir, 'LICENSE.Kenney-Animated-Characters-Retro.txt'),
    licenseText,
    'utf8',
  );
  console.log(JSON.stringify({ glbPath, contentHash, clips: manifest.clips }, null, 2));
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
