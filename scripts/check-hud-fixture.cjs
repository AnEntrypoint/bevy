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
const keys = ['dzNEXUS', 'lhAnim', 'lhPaused', 'trFade', 'trHide'];
const sampleUntil = 6.5;

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

function backdrop(png, rect) {
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
    return JSON.parse(execFileSync('python3', ['-I', '-c', probe, path.join(repo, png), JSON.stringify(rect)], { encoding: 'utf8' }));
}

function samplesOf(status) {
    return (status?.uiAnimations || [])
        .filter((entry) => keys.includes(entry.key))
        .map((entry) => ({ key: entry.key, elapsed: entry.elapsed, transitionElapsed: entry.transitionElapsed ?? null, opacity: entry.opacity, visible: entry.visible, paused: entry.paused, clock: status.uiClock }));
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
    let lastClock = null;
    try {
        const deadline = Date.now() + 180000;
        while (Date.now() < deadline && !exited && !samplesOf(readStatus()).some((s) => s.key === 'dzNEXUS')) await sleep(15);
        while (Date.now() < deadline && !exited) {
            const status = readStatus();
            const current = samplesOf(status);
            const banner = current.find((s) => s.key === 'dzNEXUS');
            if (current.length && status.uiClock !== lastClock) {
                lastClock = status.uiClock;
                samples.push(...current);
            }
            if (banner && banner.elapsed >= 1.0 && !shots.visible) {
                shots.visible = screenshot('hud-fixture-1s');
                shots.bannerRect = (status.uiLabelBoxes || []).find((box) => box.text === 'NEXUS')?.rect ?? null;
            }
            if (banner && banner.elapsed >= sampleUntil) {
                shots.ended = screenshot('hud-fixture-6s');
                break;
            }
            await sleep(banner && banner.elapsed < 0.5 ? 15 : 40);
        }
        const status = readStatus() || {};
        const positions = status.uiAnchorPositions || [];
        const viewport = status.uiAnchorViewport || [0, 0];
        const inside = positions.filter((p) => p.x >= 0 && p.y >= 0 && p.x <= viewport[0] && p.y <= viewport[1]);
        const placedIds = positions.map((p) => p.id).sort();
        result.anchors = { inScene: status.uiAnchorsInScene, placed: status.uiAnchorsPlaced, unresolved: status.uiAnchorsUnresolved, behind: status.uiAnchorsBehind, offscreen: status.uiAnchorsOffscreen, viewport, insideViewport: inside.length, placedIds, positions };
        result.labels = { count: status.uiLabels, overlaps: status.uiLabelOverlaps, uncovered: status.uiUncoveredGlyphs };
        result.screenshots = shots;
        if (shots.visible && shots.bannerRect) result.bannerBackdrop = backdrop(shots.visible, shots.bannerRect);
        result.samples = samples;
        const of = (key) => samples.filter((s) => s.key === key);
        const banner = of('dzNEXUS');
        const holding = banner.filter((s) => s.elapsed >= 0.7 && s.elapsed <= 2.9);
        const after = banner.filter((s) => s.elapsed >= 4.0);
        result.banner = { samples: banner.length, first: banner[0] ?? null, holdingSamples: holding.length, holdingAllOpaque: holding.length > 0 && holding.every((s) => s.opacity === 1 && s.visible), afterFourSamples: after.length, afterFourAllHidden: after.length > 0 && after.every((s) => s.opacity === 0 && !s.visible) };
        const longhand = of('lhAnim');
        const paused = of('lhPaused');
        const fade = of('trFade');
        const hide = of('trHide');
        const settledFade = fade.filter((s) => s.transitionElapsed !== null && s.transitionElapsed >= 1.6);
        const settledHide = hide.filter((s) => s.transitionElapsed !== null && s.transitionElapsed >= 1.1);
        const midHide = hide.filter((s) => s.transitionElapsed !== null && s.transitionElapsed > 0.05 && s.transitionElapsed < 0.95);
        result.cases = {
            longhand: { samples: longhand.length, beforeDelay: longhand.filter((s) => s.elapsed < 0.5).map((s) => s.opacity).slice(0, 3), afterEnd: longhand.filter((s) => s.elapsed >= 4.6).map((s) => s.opacity).slice(0, 3) },
            paused: { samples: paused.length, allPaused: paused.every((s) => s.paused), opacities: [...new Set(paused.map((s) => Number(s.opacity.toFixed(4))))] },
            fade: { samples: fade.length, transitionSamples: fade.filter((s) => s.transitionElapsed !== null).length, settled: settledFade.map((s) => s.opacity).slice(0, 3) },
            hide: { samples: hide.length, transitionSamples: hide.filter((s) => s.transitionElapsed !== null).length, midVisible: midHide.length > 0 && midHide.every((s) => s.visible), settled: settledHide.map((s) => [s.opacity, s.visible]).slice(0, 3) },
        };
        const anchorsOk = status.uiAnchorsInScene === expectedAnchors.length && status.uiAnchorsPlaced === expectedAnchors.length && inside.length === expectedAnchors.length && JSON.stringify(placedIds) === JSON.stringify([...expectedAnchors].sort());
        const bannerOk = Boolean(result.banner.first && result.banner.first.elapsed < 0.2 && result.banner.holdingAllOpaque && result.banner.afterFourAllHidden);
        const casesOk = longhand.length > 0
            && longhand.filter((s) => s.elapsed < 0.5).every((s) => Math.abs(s.opacity - 1) < 1e-4)
            && longhand.filter((s) => s.elapsed >= 4.6).length > 0
            && longhand.filter((s) => s.elapsed >= 4.6).every((s) => Math.abs(s.opacity - 0.2) < 1e-4)
            && paused.length > 0 && paused.every((s) => s.paused && Math.abs(s.opacity - 0.3) < 1e-4)
            && settledFade.length > 0 && settledFade.every((s) => Math.abs(s.opacity - 0.25) < 1e-4)
            && settledHide.length > 0 && settledHide.every((s) => s.opacity === 0 && !s.visible)
            && result.cases.hide.midVisible;
        result.anchorsOk = anchorsOk;
        result.bannerOk = bannerOk;
        result.casesOk = casesOk;
        result.ok = anchorsOk && bannerOk && casesOk && status.uiLabelOverlaps === 0 && Boolean(shots.visible && shots.ended) && Boolean(result.bannerBackdrop && result.bannerBackdrop.maxChannelDelta <= 12);
    } catch (error) {
        result.error = String(error.stack || error.message || error);
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
