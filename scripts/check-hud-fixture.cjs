#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const { spawn, execFileSync } = require('node:child_process');

const repo = path.resolve(__dirname, '..');
const world = process.env.WORLD || path.join(repo, 'testdata', 'hud-fixture');
const place = 'fixture';
const binary = process.env.BINARY || path.join(repo, 'target', 'debug', 'bevy-jayzee');
const display = process.env.DISPLAY || ':1';
const eye = process.env.VIEWER_EYE || '0,40,1';
const witnessDir = path.join(repo, 'target', 'witness');
const statusPath = path.join(repo, 'web', `status-${place}.json`);
const outPath = path.join(witnessDir, 'hud-fixture-native.json');
const expectedAnchors = ['fixture-district', 'fixture-event', 'fixture-foe', 'fixture-pad', 'fixture-ping'];
const bannerKey = 'dzNEXUS';
const sampleUntil = 6.0;

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

function readStatus() {
    try {
        return JSON.parse(fs.readFileSync(statusPath, 'utf8'));
    } catch {
        return null;
    }
}

function screenshot(name) {
    const tree = execFileSync('xwininfo', ['-root', '-tree'], { encoding: 'utf8', env: { ...process.env, DISPLAY: display } });
    const line = tree.split('\n').find((row) => row.includes('bevy-jayzee viewer'));
    if (!line) return null;
    const id = line.trim().split(/\s+/)[0];
    const xwd = path.join(witnessDir, `${name}.xwd`);
    const png = path.join(witnessDir, `${name}.png`);
    execFileSync('xwd', ['-id', id, '-silent', '-out', xwd], { env: { ...process.env, DISPLAY: display } });
    execFileSync('python3', ['-I', path.join(repo, 'scripts', 'xwd2png.py'), xwd, png]);
    return path.relative(repo, png);
}

function bannerSample(status) {
    const sample = (status?.uiAnimations || []).find((entry) => entry.key === bannerKey);
    return sample ? { elapsed: sample.elapsed, opacity: sample.opacity, visible: sample.visible, clock: status.uiClock } : null;
}

async function main() {
    fs.mkdirSync(witnessDir, { recursive: true });
    fs.rmSync(statusPath, { force: true });
    const result = { ok: false, world: path.relative(repo, world), place, eye };
    const viewer = spawn(binary, ['viewer', world, place], {
        cwd: repo,
        env: { ...process.env, DISPLAY: display, VIEWER_EYE: eye },
        stdio: ['ignore', fs.openSync(path.join(witnessDir, 'hud-fixture-viewer.out'), 'w'), fs.openSync(path.join(witnessDir, 'hud-fixture-viewer.err'), 'w')],
    });
    result.viewerPid = viewer.pid;
    let exited = false;
    viewer.on('exit', () => {
        exited = true;
    });
    const samples = [];
    const shots = {};
    try {
        const deadline = Date.now() + 180000;
        while (Date.now() < deadline && !exited && !bannerSample(readStatus())) await sleep(20);
        while (Date.now() < deadline && !exited) {
            const status = readStatus();
            const sample = bannerSample(status);
            if (sample && (samples.length === 0 || sample.clock !== samples[samples.length - 1].clock)) samples.push(sample);
            if (sample && sample.elapsed >= 1.0 && !shots.visible) {
                shots.visible = screenshot('hud-fixture-1s');
                shots.bannerRect = ((status && status.uiLabelBoxes) || []).find((box) => box.text === 'NEXUS')?.rect ?? null;
            }
            if (sample && sample.elapsed >= sampleUntil) {
                shots.ended = screenshot('hud-fixture-6s');
                break;
            }
            await sleep(sample && sample.elapsed < 0.5 ? 15 : 60);
        }
        const status = readStatus() || {};
        const positions = status.uiAnchorPositions || [];
        const viewport = status.uiAnchorViewport || [0, 0];
        const inside = positions.filter((p) => p.x >= 0 && p.y >= 0 && p.x <= viewport[0] && p.y <= viewport[1]);
        const placedIds = positions.map((p) => p.id).sort();
        result.anchors = {
            inScene: status.uiAnchorsInScene,
            placed: status.uiAnchorsPlaced,
            unresolved: status.uiAnchorsUnresolved,
            behind: status.uiAnchorsBehind,
            offscreen: status.uiAnchorsOffscreen,
            viewport,
            insideViewport: inside.length,
            placedIds,
            positions,
        };
        result.labels = { count: status.uiLabels, overlaps: status.uiLabelOverlaps, uncovered: status.uiUncoveredGlyphs };
        result.screenshots = shots;
        if (shots.visible && shots.bannerRect) {
            const probe = `
from PIL import Image
import json, sys
im = Image.open(sys.argv[1]).convert('RGB')
l, t, r, b = [int(round(v)) for v in json.loads(sys.argv[2])]
inside = [im.getpixel((x, y)) for x, y in [(l + 1, t + 1), (r - 2, t + 1), (l + 1, b - 2), (r - 2, b - 2)]]
outside = [im.getpixel((x, y)) for x, y in [(l - 4, t + 1), (r + 3, t + 1), (l - 4, b - 2), (r + 3, b - 2)]]
delta = max(abs(a - c) for p, q in zip(inside, outside) for a, c in zip(p, q))
print(json.dumps({'inside': inside, 'outside': outside, 'maxChannelDelta': delta}))
`;
            result.bannerBackdrop = JSON.parse(execFileSync('python3', ['-I', '-c', probe, path.join(repo, shots.visible), JSON.stringify(shots.bannerRect)], { encoding: 'utf8' }));
        }
        result.samples = samples;
        const first = samples[0];
        const holding = samples.filter((s) => s.elapsed >= 0.7 && s.elapsed <= 2.9);
        const after = samples.filter((s) => s.elapsed >= 4.0);
        result.banner = {
            samples: samples.length,
            first,
            holdingSamples: holding.length,
            holdingAllOpaque: holding.length > 0 && holding.every((s) => s.opacity === 1 && s.visible),
            afterFourSamples: after.length,
            afterFourAllHidden: after.length > 0 && after.every((s) => s.opacity === 0 && !s.visible),
            lastBeforeFour: samples.filter((s) => s.elapsed < 4.0).pop() || null,
        };
        const anchorsOk = status.uiAnchorsInScene === expectedAnchors.length
            && status.uiAnchorsPlaced === expectedAnchors.length
            && inside.length === expectedAnchors.length
            && JSON.stringify(placedIds) === JSON.stringify([...expectedAnchors].sort());
        const bannerOk = Boolean(first && first.elapsed < 0.2 && result.banner.holdingAllOpaque && result.banner.afterFourAllHidden);
        result.anchorsOk = anchorsOk;
        result.bannerOk = bannerOk;
        result.ok = anchorsOk && bannerOk && status.uiLabelOverlaps === 0 && Boolean(shots.visible && shots.ended) && Boolean(result.bannerBackdrop && result.bannerBackdrop.maxChannelDelta <= 12);
    } catch (error) {
        result.error = String(error.message || error);
    } finally {
        if (!exited) {
            viewer.kill('SIGTERM');
            const stopBy = Date.now() + 10000;
            while (!exited && Date.now() < stopBy) await sleep(100);
            if (!exited) viewer.kill('SIGKILL');
        }
        result.viewerStopped = exited || viewer.killed;
    }
    fs.writeFileSync(outPath, JSON.stringify(result, null, 2));
    return result;
}

main().then((result) => {
    const { samples, ...brief } = result;
    console.log(JSON.stringify({ ...brief, samples: samples ? samples.length : 0, witness: outPath }, null, 2));
    process.exit(result.ok ? 0 : 1);
});
