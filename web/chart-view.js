// Draws a measurement chart as SVG from the proven numbers in charts.ts.
// DOM code, so not proven itself; all data goes in as text or numbers.
import { niceTop, peakOf, strokes, xOf, yOf } from './charts.js';
const SVG = 'http://www.w3.org/2000/svg';
function node(tag, attrs, text) {
    const e = document.createElementNS(SVG, tag);
    for (const [k, v] of Object.entries(attrs))
        e.setAttribute(k, String(v));
    if (text !== undefined)
        e.textContent = text;
    return e;
}
const HEIGHT = 96;
const LEFT = 52;
const TOP = 22;
const BOTTOM = 20;
/** A small line chart: the axis top, the times at both ends, one line per series. */
export function drawChart(c) {
    const n = Math.max(1, ...c.lines.map((l) => l.values.length));
    const w = Math.max(120, c.width - LEFT - 6);
    const peak = peakOf(c.lines.flatMap((l) => l.values.map((v) => (v === null ? 0 : Math.ceil(v)))));
    const top = niceTop(peak);
    const total = TOP + HEIGHT + BOTTOM;
    const s = node('svg', { class: 'chart', width: c.width, height: total, viewBox: `0 0 ${c.width} ${total}`, role: 'img', 'aria-label': c.title });
    s.append(node('text', { x: 0, y: 14, class: 'chart-title' }, c.title));
    s.append(node('text', { x: LEFT - 6, y: TOP + 4, 'text-anchor': 'end', class: 'chart-axis' }, c.unit(top)));
    s.append(node('text', { x: LEFT - 6, y: TOP + HEIGHT, 'text-anchor': 'end', class: 'chart-axis' }, c.unit(0)));
    s.append(node('line', { x1: LEFT, x2: LEFT + w, y1: TOP, y2: TOP, class: 'chart-grid' }));
    s.append(node('line', { x1: LEFT, x2: LEFT + w, y1: TOP + HEIGHT, y2: TOP + HEIGHT, class: 'chart-base' }));
    s.append(node('text', { x: LEFT, y: total - 4, class: 'chart-axis' }, c.start));
    s.append(node('text', { x: LEFT + w, y: total - 4, 'text-anchor': 'end', class: 'chart-axis' }, c.end));
    for (const l of c.lines) {
        const how = strokes(l.values.map((v) => v !== null));
        let d = '';
        l.values.forEach((v, i) => {
            if (v === null || how[i] === 0)
                return;
            const x = (LEFT + xOf(i, n, w)).toFixed(1);
            const y = (TOP + yOf(v, top, HEIGHT)).toFixed(1);
            d += `${how[i] === 1 ? 'M' : 'L'}${x} ${y} `;
            // A lone measurement has no line to show it; a dot does.
            if (how[i] === 1 && how[i + 1] !== 2)
                s.append(node('circle', { cx: x, cy: y, r: 1.5, class: `chart-dot ${l.cls}` }));
        });
        if (d)
            s.append(node('path', { d: d.trim(), class: `chart-line ${l.cls}` }));
    }
    return s;
}
