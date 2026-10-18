// The numbers behind the measurement charts: how high the axis reaches, where
// a value sits in the chart, and which measured points are joined by a line.
// Pure logic with contracts checked by LemmaScript (lsc); the app draws the
// SVG from it. The proofs take values as real numbers; at runtime they are
// floating point, which can only round, not move a point out of the chart.
/** The largest value, and at least 0 (whole numbers: the app rounds up). */
export function peakOf(values) {
    //@ verify
    //@ ensures \result >= 0
    //@ ensures forall(k, 0 <= k && k < values.length ==> values[k] <= \result)
    let top = 0;
    let i = 0;
    while (i < values.length) {
        //@ invariant 0 <= i && i <= values.length
        //@ invariant top >= 0
        //@ invariant forall(k, 0 <= k && k < i ==> values[k] <= top)
        //@ decreases values.length - i
        if (values[i] > top)
            top = values[i];
        i = i + 1;
    }
    return top;
}
/** The axis top: the smallest of 1, 2, 5, 10, 20, 50, … that is at least `peak`. */
export function niceTop(peak) {
    //@ verify
    //@ requires peak >= 0
    //@ ensures \result >= peak && \result >= 1
    let base = 1;
    let k = 0;
    while (k < 40 && base * 5 < peak) {
        //@ invariant base >= 1
        //@ invariant 0 <= k && k <= 40
        //@ decreases 40 - k
        base = base * 10;
        k = k + 1;
    }
    if (base >= peak)
        return base;
    if (base * 2 >= peak)
        return base * 2;
    if (base * 5 >= peak)
        return base * 5;
    return peak;
}
/** How far down a chart `h` high the value `v` sits, with `top` at the top. */
export function yOf(v, top, h) {
    //@ verify
    //@ requires top > 0 && h >= 0
    //@ ensures 0 <= \result && \result <= h
    //@ ensures v <= 0 ==> \result === h
    //@ ensures v >= top ==> \result === 0
    if (v <= 0)
        return h;
    if (v >= top)
        return 0 * h;
    const part = v / top;
    return h - part * h;
}
/** Where the `i`-th of `n` points sits across a chart `w` wide. */
export function xOf(i, n, w) {
    //@ verify
    //@ requires 0 <= i && i < n && w >= 0
    //@ ensures 0 <= \result && \result <= w
    //@ ensures i === 0 ==> \result === 0
    if (n === 1)
        return 0 * w;
    const part = i / (n - 1);
    return part * w;
}
/**
 * How each point is drawn: 0 not at all (nothing measured), 1 starts a line,
 * 2 continues the line from the point before. A line joins only two
 * neighbouring measured points, so a gap in the measurements stays a gap.
 */
export function strokes(present) {
    //@ verify
    //@ ensures \result.length === present.length
    //@ ensures forall(k, 0 <= k && k < present.length ==> (\result[k] === 0) === !present[k])
    //@ ensures forall(k, 0 <= k && k < present.length ==> \result[k] === 0 || \result[k] === 1 || \result[k] === 2)
    //@ ensures forall(k, 0 <= k && k < present.length && \result[k] === 2 ==> k > 0 && present[k - 1])
    //@ ensures forall(k, 0 < k && k < present.length && present[k] && present[k - 1] ==> \result[k] === 2)
    const out = [];
    let i = 0;
    while (i < present.length) {
        //@ invariant 0 <= i && i <= present.length
        //@ invariant out.length === i
        //@ invariant forall(k, 0 <= k && k < i ==> (out[k] === 0) === !present[k])
        //@ invariant forall(k, 0 <= k && k < i ==> out[k] === 0 || out[k] === 1 || out[k] === 2)
        //@ invariant forall(k, 0 <= k && k < i && out[k] === 2 ==> k > 0 && present[k - 1])
        //@ invariant forall(k, 0 < k && k < i && present[k] && present[k - 1] ==> out[k] === 2)
        //@ decreases present.length - i
        if (!present[i])
            out.push(0);
        else if (i > 0 && present[i - 1])
            out.push(2);
        else
            out.push(1);
        i = i + 1;
    }
    return out;
}
